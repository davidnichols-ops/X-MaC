// X-MaC Digital Twin Database — persistent event-sourced storage
//
// The SQLite database is the source of truth. The KnowledgeGraph is a
// materialized view built from events. This enables temporal queries:
// "Show me the state of my Mac yesterday at 3 PM."
//
// Schema:
//   events        — raw event log (retention: 7 days)
//   entities      — current entity state (apps, files, processes, etc.)
//   relationships — graph edges between entities
//   snapshots     — full graph snapshots for quick point-in-time restore
//   aggregates    — time-bucketed summaries (hourly: 90d, daily: forever)
//
// EntityStore and GraphStore are pre-built for Phase 1b (observers) and
// are not yet called from the binary. They are intentionally dead code
// until observers are implemented.

#![allow(dead_code, unused_imports)]

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod entity_store;
pub mod event_store;
pub mod graph_store;

pub use entity_store::EntityStore;
pub use event_store::EventStore;
pub use graph_store::GraphStore;

/// Default location for the twin database.
pub fn default_db_path() -> Result<std::path::PathBuf> {
    let dir = dirs::data_local_dir().context("no local data dir")?;
    let dir = dir.join("x-mac");
    std::fs::create_dir_all(&dir).context("create data dir")?;
    Ok(dir.join("twin.db"))
}

/// Wrapper around a SQLite connection with WAL mode and pragmas tuned for
/// write-heavy event ingestion.
pub struct TwinDb {
    conn: Arc<Mutex<Connection>>,
}

impl TwinDb {
    /// Open (or create) the twin database at `path` and run migrations.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("open twin db at {}", path.display()))?;

        // WAL mode + tuned pragmas for concurrent read/write.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;
             PRAGMA cache_size = -65536;  -- 64MB page cache
             PRAGMA temp_store = MEMORY;",
        )
        .context("set pragmas")?;

        Self::run_migrations(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory database (for tests).
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "PRAGMA journal_mode = MEMORY;
             PRAGMA synchronous = OFF;
             PRAGMA foreign_keys = ON;",
        )?;
        Self::run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn run_migrations(conn: &Connection) -> Result<()> {
        // Migration v1: initial schema.
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- ── events: raw event log ──────────────────────────────────
            CREATE TABLE IF NOT EXISTS events (
                id          TEXT PRIMARY KEY,
                timestamp   INTEGER NOT NULL,
                event_type  TEXT NOT NULL,
                severity    TEXT NOT NULL,
                source      TEXT NOT NULL,
                entity_id   TEXT,
                payload     TEXT NOT NULL DEFAULT '{}'
            );
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_entity   ON events(entity_id);
            CREATE INDEX IF NOT EXISTS idx_events_type     ON events(event_type);
            CREATE INDEX IF NOT EXISTS idx_events_severity ON events(severity);

            -- ── entities: current state of tracked objects ─────────────
            CREATE TABLE IF NOT EXISTS entities (
                id            TEXT PRIMARY KEY,
                entity_type   TEXT NOT NULL,
                name          TEXT NOT NULL,
                path          TEXT,
                created_at    INTEGER NOT NULL,
                updated_at    INTEGER NOT NULL,
                metadata      TEXT NOT NULL DEFAULT '{}'
            );
            CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name);

            -- ── relationships: graph edges ─────────────────────────────
            CREATE TABLE IF NOT EXISTS relationships (
                id                TEXT PRIMARY KEY,
                source_entity     TEXT NOT NULL,
                target_entity     TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                created_at        INTEGER NOT NULL,
                last_seen         INTEGER NOT NULL,
                metadata          TEXT NOT NULL DEFAULT '{}',
                FOREIGN KEY (source_entity) REFERENCES entities(id) ON DELETE CASCADE,
                FOREIGN KEY (target_entity) REFERENCES entities(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_rel_source ON relationships(source_entity);
            CREATE INDEX IF NOT EXISTS idx_rel_target ON relationships(target_entity);
            CREATE INDEX IF NOT EXISTS idx_rel_type   ON relationships(relationship_type);

            -- ── snapshots: full graph point-in-time ────────────────────
            CREATE TABLE IF NOT EXISTS snapshots (
                id            TEXT PRIMARY KEY,
                timestamp     INTEGER NOT NULL,
                graph_version INTEGER NOT NULL,
                snapshot      TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_ts ON snapshots(timestamp);

            -- ── aggregates: time-bucketed summaries ────────────────────
            CREATE TABLE IF NOT EXISTS aggregates (
                id          TEXT PRIMARY KEY,
                time_bucket INTEGER NOT NULL,   -- epoch seconds aligned to bucket
                bucket_size TEXT NOT NULL,       -- 'hour' | 'day'
                metric_type TEXT NOT NULL,
                entity_id   TEXT,
                value       REAL NOT NULL,
                metadata    TEXT NOT NULL DEFAULT '{}'
            );
            CREATE INDEX IF NOT EXISTS idx_agg_bucket ON aggregates(time_bucket);
            CREATE INDEX IF NOT EXISTS idx_agg_type   ON aggregates(metric_type);
            CREATE INDEX IF NOT EXISTS idx_agg_entity ON aggregates(entity_id);

            -- Record migration.
            INSERT OR IGNORE INTO schema_version (version) VALUES (1);
            "#,
        )
        .context("run migration v1")?;

        Ok(())
    }

    /// Acquire the connection lock. Use this for direct access.
    pub async fn conn(&self) -> tokio::sync::MutexGuard<'_, Connection> {
        self.conn.lock().await
    }

    /// Get a cloned handle (shares the same connection mutex).
    pub fn handle(&self) -> TwinDbHandle {
        TwinDbHandle {
            conn: self.conn.clone(),
        }
    }

    /// Close the database (flush WAL).
    pub fn close(self) -> Result<()> {
        // Arc<Mutex> will drop naturally; explicit checkpoint happens on close.
        Ok(())
    }
}

/// A lightweight handle that shares the underlying connection.
/// Useful for passing to stores without transferring ownership.
#[derive(Clone)]
pub struct TwinDbHandle {
    conn: Arc<Mutex<Connection>>,
}

impl TwinDbHandle {
    pub async fn conn(&self) -> tokio::sync::MutexGuard<'_, Connection> {
        self.conn.lock().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_open_memory() {
        let db = TwinDb::open_memory().unwrap();
        let conn = db.conn().await;
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 1);
    }

    #[tokio::test]
    async fn test_tables_exist() {
        let db = TwinDb::open_memory().unwrap();
        let conn = db.conn().await;
        for table in [
            "events",
            "entities",
            "relationships",
            "snapshots",
            "aggregates",
        ] {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                        table
                    ),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table {} should exist", table);
        }
    }
}
