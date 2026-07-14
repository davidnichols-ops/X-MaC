// EventStore — append-only event log backed by SQLite
//
// Events are the source of truth. Every state change in the twin is recorded
// as an event. The graph is a materialized view derived from events.
//
// Retention:
//   raw events:     7 days
//   hourly aggregates: 90 days
//   daily aggregates:  forever
//
// Compaction merges raw events into aggregates once they age out.

use super::TwinDbHandle;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single event in the twin's timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub id: String,
    pub timestamp_ms: i64,
    pub event_type: String,
    pub severity: String,
    pub source: String,
    pub entity_id: Option<String>,
    pub payload: HashMap<String, serde_json::Value>,
}

/// Query filter for events.
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub event_type: Option<String>,
    pub entity_id: Option<String>,
    pub severity: Option<String>,
    pub source: Option<String>,
    pub limit: Option<usize>,
}

pub struct EventStore {
    db: TwinDbHandle,
}

impl EventStore {
    pub fn new(db: TwinDbHandle) -> Self {
        Self { db }
    }

    /// Append a single event. Generates a UUIDv7 if id is empty.
    pub async fn append_event(&self, mut event: StoredEvent) -> Result<String> {
        if event.id.is_empty() {
            event.id = uuid::Uuid::now_v7().to_string();
        }
        let payload_json = serde_json::to_string(&event.payload)?;
        let conn = self.db.conn().await;
        conn.execute(
            "INSERT INTO events (id, timestamp, event_type, severity, source, entity_id, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.id,
                event.timestamp_ms,
                event.event_type,
                event.severity,
                event.source,
                event.entity_id,
                payload_json,
            ],
        )?;
        Ok(event.id)
    }

    /// Append a batch of events in a single transaction.
    pub async fn append_batch(&self, events: Vec<StoredEvent>) -> Result<usize> {
        let mut conn = self.db.conn().await;
        let tx = conn.transaction()?;
        let mut count = 0;
        for mut event in events {
            if event.id.is_empty() {
                event.id = uuid::Uuid::now_v7().to_string();
            }
            let payload_json = serde_json::to_string(&event.payload)?;
            tx.execute(
                "INSERT INTO events (id, timestamp, event_type, severity, source, entity_id, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    event.id,
                    event.timestamp_ms,
                    event.event_type,
                    event.severity,
                    event.source,
                    event.entity_id,
                    payload_json,
                ],
            )?;
            count += 1;
        }
        tx.commit()?;
        Ok(count)
    }

    /// Query events with optional filters, ordered by timestamp descending.
    pub async fn query_events(&self, query: &EventQuery) -> Result<Vec<StoredEvent>> {
        let conn = self.db.conn().await;
        let mut sql = String::from(
            "SELECT id, timestamp, event_type, severity, source, entity_id, payload
             FROM events WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref et) = query.event_type {
            sql.push_str(&format!(" AND event_type = ?{}", param_values.len() + 1));
            param_values.push(Box::new(et.clone()));
        }
        if let Some(ref eid) = query.entity_id {
            sql.push_str(&format!(" AND entity_id = ?{}", param_values.len() + 1));
            param_values.push(Box::new(eid.clone()));
        }
        if let Some(ref sev) = query.severity {
            sql.push_str(&format!(" AND severity = ?{}", param_values.len() + 1));
            param_values.push(Box::new(sev.clone()));
        }
        if let Some(ref src) = query.source {
            sql.push_str(&format!(" AND source = ?{}", param_values.len() + 1));
            param_values.push(Box::new(src.clone()));
        }
        sql.push_str(" ORDER BY timestamp DESC");
        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let payload_str: String = row.get(6)?;
            let payload: HashMap<String, serde_json::Value> =
                serde_json::from_str(&payload_str).unwrap_or_default();
            Ok(StoredEvent {
                id: row.get(0)?,
                timestamp_ms: row.get(1)?,
                event_type: row.get(2)?,
                severity: row.get(3)?,
                source: row.get(4)?,
                entity_id: row.get(5)?,
                payload,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Get all events between two timestamps (inclusive), ascending order.
    pub async fn get_events_between(&self, start_ms: i64, end_ms: i64) -> Result<Vec<StoredEvent>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, event_type, severity, source, entity_id, payload
             FROM events
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp ASC",
        )?;
        let rows = stmt.query_map(params![start_ms, end_ms], |row| {
            let payload_str: String = row.get(6)?;
            let payload: HashMap<String, serde_json::Value> =
                serde_json::from_str(&payload_str).unwrap_or_default();
            Ok(StoredEvent {
                id: row.get(0)?,
                timestamp_ms: row.get(1)?,
                event_type: row.get(2)?,
                severity: row.get(3)?,
                source: row.get(4)?,
                entity_id: row.get(5)?,
                payload,
            })
        })?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Compact raw events older than `retention_days` into hourly aggregates,
    /// then delete the raw events. Returns (compacted_count, deleted_count).
    pub async fn compact_events(&self, retention_days: i64) -> Result<(usize, usize)> {
        let cutoff_ms = (Utc::now() - Duration::days(retention_days)).timestamp_millis();
        let mut conn = self.db.conn().await;

        // Build hourly aggregates per (event_type, entity_id, hour_bucket).
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO aggregates (id, time_bucket, bucket_size, metric_type, entity_id, value, metadata)
             SELECT
                ?1 || ':' || event_type || ':' || COALESCE(entity_id, '*') || ':' || (timestamp / 3600000),
                (timestamp / 3600000) * 3600,
                'hour',
                event_type,
                entity_id,
                COUNT(*) as count,
                json_object('compacted_from', MIN(timestamp), 'compacted_to', MAX(timestamp))
             FROM events
             WHERE timestamp < ?2
             GROUP BY event_type, entity_id, (timestamp / 3600000)
             ON CONFLICT(id) DO UPDATE SET
                value = value + excluded.value,
                metadata = excluded.metadata",
            params![uuid::Uuid::now_v7().to_string(), cutoff_ms],
        )?;

        let deleted: usize = tx.execute(
            "DELETE FROM events WHERE timestamp < ?1",
            params![cutoff_ms],
        )?;
        tx.commit()?;

        // Now compact hourly aggregates older than 90 days into daily.
        // Reuse the same connection (still held).
        let daily_cutoff_ms = (Utc::now() - Duration::days(90)).timestamp_millis();
        let tx2 = conn.transaction()?;
        tx2.execute(
            "INSERT INTO aggregates (id, time_bucket, bucket_size, metric_type, entity_id, value, metadata)
             SELECT
                ?1 || ':' || metric_type || ':' || COALESCE(entity_id, '*') || ':' || (time_bucket / 86400),
                (time_bucket / 86400) * 86400,
                'day',
                metric_type,
                entity_id,
                SUM(value),
                json_object('compacted_from_hourly', MIN(time_bucket), 'compacted_to_hourly', MAX(time_bucket))
             FROM aggregates
             WHERE bucket_size = 'hour' AND time_bucket * 1000 < ?2
             GROUP BY metric_type, entity_id, (time_bucket / 86400)
             ON CONFLICT(id) DO UPDATE SET
                value = value + excluded.value",
            params![uuid::Uuid::now_v7().to_string(), daily_cutoff_ms],
        )?;
        let deleted_hourly: usize = tx2.execute(
            "DELETE FROM aggregates WHERE bucket_size = 'hour' AND time_bucket * 1000 < ?1",
            params![daily_cutoff_ms],
        )?;
        tx2.commit()?;

        Ok((deleted_hourly, deleted))
    }

    /// Get event count (for diagnostics).
    pub async fn event_count(&self) -> Result<i64> {
        let conn = self.db.conn().await;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
        Ok(count)
    }

    /// Get aggregate count.
    pub async fn aggregate_count(&self) -> Result<i64> {
        let conn = self.db.conn().await;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM aggregates", [], |r| r.get(0))?;
        Ok(count)
    }
}

/// Helper to convert a DateTime to millis for storage.
pub fn to_millis(dt: DateTime<Utc>) -> i64 {
    dt.timestamp_millis()
}

/// Helper to convert stored millis back to DateTime.
pub fn from_millis(ms: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twin::database::TwinDb;

    async fn setup() -> (TwinDb, EventStore) {
        let db = TwinDb::open_memory().unwrap();
        let store = EventStore::new(db.handle());
        (db, store)
    }

    fn make_event(et: &str, ts_ms: i64) -> StoredEvent {
        StoredEvent {
            id: String::new(),
            timestamp_ms: ts_ms,
            event_type: et.to_string(),
            severity: "info".to_string(),
            source: "test".to_string(),
            entity_id: Some("entity-1".to_string()),
            payload: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_append_and_query() {
        let (_db, store) = setup().await;
        store
            .append_event(make_event("ProcessLaunched", 1000))
            .await
            .unwrap();
        store
            .append_event(make_event("ProcessTerminated", 2000))
            .await
            .unwrap();

        let all = store.query_events(&EventQuery::default()).await.unwrap();
        assert_eq!(all.len(), 2);
        // Descending order — most recent first.
        assert_eq!(all[0].timestamp_ms, 2000);
    }

    #[tokio::test]
    async fn test_batch_append() {
        let (_db, store) = setup().await;
        let events = vec![
            make_event("FileCreated", 1000),
            make_event("FileCreated", 1100),
            make_event("FileCreated", 1200),
        ];
        let count = store.append_batch(events).await.unwrap();
        assert_eq!(count, 3);
        assert_eq!(store.event_count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_get_between() {
        let (_db, store) = setup().await;
        store.append_event(make_event("A", 1000)).await.unwrap();
        store.append_event(make_event("B", 2000)).await.unwrap();
        store.append_event(make_event("C", 3000)).await.unwrap();

        let mid = store.get_events_between(1500, 2500).await.unwrap();
        assert_eq!(mid.len(), 1);
        assert_eq!(mid[0].event_type, "B");
    }

    #[tokio::test]
    async fn test_compaction() {
        let (_db, store) = setup().await;
        // Insert events far in the past.
        let old_ms = (Utc::now() - Duration::days(30)).timestamp_millis();
        for i in 0..10 {
            store
                .append_event(make_event("FileCreated", old_ms + i * 1000))
                .await
                .unwrap();
        }
        assert_eq!(store.event_count().await.unwrap(), 10);

        // Compact events older than 7 days.
        let (_hourly_deleted, raw_deleted) = store.compact_events(7).await.unwrap();
        assert_eq!(raw_deleted, 10);
        assert_eq!(store.event_count().await.unwrap(), 0);
        // Aggregates should have been created.
        assert!(store.aggregate_count().await.unwrap() > 0);
    }
}
