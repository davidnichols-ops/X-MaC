// EntityStore — current state of tracked objects (apps, files, processes, etc.)
//
// Entities are the nodes of the knowledge graph. Their current state is
// derived from the event log but cached here for fast lookup. When an event
// arrives, the entity's `updated_at` and `metadata` are updated.

use super::TwinDbHandle;
use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A tracked entity in the twin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub path: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct EntityStore {
    db: TwinDbHandle,
}

impl EntityStore {
    pub fn new(db: TwinDbHandle) -> Self {
        Self { db }
    }

    /// Create a new entity. Fails if the id already exists.
    pub async fn create_entity(&self, entity: Entity) -> Result<String> {
        let metadata_json = serde_json::to_string(&entity.metadata)?;
        let conn = self.db.conn().await;
        conn.execute(
            "INSERT INTO entities (id, entity_type, name, path, created_at, updated_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entity.id,
                entity.entity_type,
                entity.name,
                entity.path,
                entity.created_at_ms,
                entity.updated_at_ms,
                metadata_json,
            ],
        )?;
        Ok(entity.id)
    }

    /// Upsert: create or update an entity. On conflict, updates name, path,
    /// updated_at, and merges metadata.
    pub async fn upsert_entity(&self, entity: Entity) -> Result<String> {
        let metadata_json = serde_json::to_string(&entity.metadata)?;
        let conn = self.db.conn().await;
        conn.execute(
            "INSERT INTO entities (id, entity_type, name, path, created_at, updated_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                updated_at = excluded.updated_at,
                metadata = excluded.metadata",
            params![
                entity.id,
                entity.entity_type,
                entity.name,
                entity.path,
                entity.created_at_ms,
                entity.updated_at_ms,
                metadata_json,
            ],
        )?;
        Ok(entity.id)
    }

    /// Update an entity's metadata and updated_at timestamp.
    pub async fn update_entity(
        &self,
        id: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let metadata_json = serde_json::to_string(&metadata)?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let conn = self.db.conn().await;
        conn.execute(
            "UPDATE entities SET metadata = ?1, updated_at = ?2 WHERE id = ?3",
            params![metadata_json, now_ms, id],
        )?;
        Ok(())
    }

    /// Get a single entity by id.
    pub async fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, name, path, created_at, updated_at, metadata
             FROM entities WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            let metadata_str: String = row.get(6)?;
            let metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&metadata_str).unwrap_or_default();
            Ok(Entity {
                id: row.get(0)?,
                entity_type: row.get(1)?,
                name: row.get(2)?,
                path: row.get(3)?,
                created_at_ms: row.get(4)?,
                updated_at_ms: row.get(5)?,
                metadata,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Find all entities of a given type.
    pub async fn get_entities_by_type(&self, entity_type: &str) -> Result<Vec<Entity>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, name, path, created_at, updated_at, metadata
             FROM entities WHERE entity_type = ?1 ORDER BY name",
        )?;
        let rows = stmt.query_map(params![entity_type], |row| {
            let metadata_str: String = row.get(6)?;
            let metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&metadata_str).unwrap_or_default();
            Ok(Entity {
                id: row.get(0)?,
                entity_type: row.get(1)?,
                name: row.get(2)?,
                path: row.get(3)?,
                created_at_ms: row.get(4)?,
                updated_at_ms: row.get(5)?,
                metadata,
            })
        })?;
        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    /// Find entities related to `entity_id` via the relationships table.
    /// Returns (entity, relationship_type, direction) tuples.
    pub async fn find_related_entities(
        &self,
        entity_id: &str,
    ) -> Result<Vec<(Entity, String, String)>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT e.id, e.entity_type, e.name, e.path, e.created_at, e.updated_at, e.metadata,
                    r.relationship_type,
                    CASE WHEN r.source_entity = ?1 THEN 'outgoing' ELSE 'incoming' END as direction
             FROM relationships r
             JOIN entities e ON e.id = CASE WHEN r.source_entity = ?1 THEN r.target_entity ELSE r.source_entity END
             WHERE r.source_entity = ?1 OR r.target_entity = ?1
             ORDER BY r.last_seen DESC",
        )?;
        let rows = stmt.query_map(params![entity_id], |row| {
            let metadata_str: String = row.get(6)?;
            let metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&metadata_str).unwrap_or_default();
            Ok((
                Entity {
                    id: row.get(0)?,
                    entity_type: row.get(1)?,
                    name: row.get(2)?,
                    path: row.get(3)?,
                    created_at_ms: row.get(4)?,
                    updated_at_ms: row.get(5)?,
                    metadata,
                },
                row.get(7)?,
                row.get(8)?,
            ))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Delete an entity and its relationships (cascade).
    pub async fn delete_entity(&self, id: &str) -> Result<()> {
        let conn = self.db.conn().await;
        conn.execute("DELETE FROM entities WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Count entities by type.
    pub async fn count_by_type(&self) -> Result<HashMap<String, i64>> {
        let conn = self.db.conn().await;
        let mut stmt =
            conn.prepare("SELECT entity_type, COUNT(*) FROM entities GROUP BY entity_type")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut counts = HashMap::new();
        for row in rows {
            let (t, c) = row?;
            counts.insert(t, c);
        }
        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twin::database::TwinDb;

    async fn setup() -> (TwinDb, EntityStore) {
        let db = TwinDb::open_memory().unwrap();
        let store = EntityStore::new(db.handle());
        (db, store)
    }

    fn make_entity(id: &str, et: &str, name: &str) -> Entity {
        Entity {
            id: id.to_string(),
            entity_type: et.to_string(),
            name: name.to_string(),
            path: Some(format!("/apps/{}", name)),
            created_at_ms: 1000,
            updated_at_ms: 1000,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let (_db, store) = setup().await;
        store
            .create_entity(make_entity("app:xcode", "application", "Xcode"))
            .await
            .unwrap();
        let entity = store.get_entity("app:xcode").await.unwrap();
        assert!(entity.is_some());
        assert_eq!(entity.unwrap().name, "Xcode");
    }

    #[tokio::test]
    async fn test_upsert() {
        let (_db, store) = setup().await;
        let mut e = make_entity("app:chrome", "application", "Chrome");
        store.upsert_entity(e.clone()).await.unwrap();
        e.name = "Google Chrome".to_string();
        store.upsert_entity(e).await.unwrap();
        let entity = store.get_entity("app:chrome").await.unwrap().unwrap();
        assert_eq!(entity.name, "Google Chrome");
    }

    #[tokio::test]
    async fn test_get_by_type() {
        let (_db, store) = setup().await;
        store
            .create_entity(make_entity("app:a", "application", "A"))
            .await
            .unwrap();
        store
            .create_entity(make_entity("app:b", "application", "B"))
            .await
            .unwrap();
        store
            .create_entity(make_entity("file:c", "file", "C"))
            .await
            .unwrap();
        let apps = store.get_entities_by_type("application").await.unwrap();
        assert_eq!(apps.len(), 2);
    }

    #[tokio::test]
    async fn test_count_by_type() {
        let (_db, store) = setup().await;
        store
            .create_entity(make_entity("app:a", "application", "A"))
            .await
            .unwrap();
        store
            .create_entity(make_entity("app:b", "application", "B"))
            .await
            .unwrap();
        store
            .create_entity(make_entity("file:c", "file", "C"))
            .await
            .unwrap();
        let counts = store.count_by_type().await.unwrap();
        assert_eq!(counts.get("application"), Some(&2));
        assert_eq!(counts.get("file"), Some(&1));
    }
}
