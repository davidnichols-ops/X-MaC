// GraphStore — relationship edges between entities, with subgraph queries
//
// The graph is a materialized view derived from events. Relationships track
// which entities are connected (e.g., "Xcode" --created--> "file.swift").
// The GraphStore supports neighborhood queries and subgraph extraction for
// the knowledge graph views.

use super::TwinDbHandle;
use crate::twin::database::entity_store::Entity;
use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A relationship edge between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub source_entity: String,
    pub target_entity: String,
    pub relationship_type: String,
    pub created_at_ms: i64,
    pub last_seen_ms: i64,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A subgraph: nodes + edges extracted from the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subgraph {
    pub nodes: Vec<Entity>,
    pub edges: Vec<Relationship>,
    pub root_entity_id: String,
}

pub struct GraphStore {
    db: TwinDbHandle,
}

impl GraphStore {
    pub fn new(db: TwinDbHandle) -> Self {
        Self { db }
    }

    /// Add or update a relationship. On conflict (same source+target+type),
    /// updates `last_seen` and merges metadata.
    pub async fn add_relationship(&self, rel: Relationship) -> Result<String> {
        let metadata_json = serde_json::to_string(&rel.metadata)?;
        let conn = self.db.conn().await;
        // Use a deterministic id if empty.
        let id = if rel.id.is_empty() {
            format!(
                "rel:{}:{}:{}",
                rel.source_entity, rel.target_entity, rel.relationship_type
            )
        } else {
            rel.id
        };
        conn.execute(
            "INSERT INTO relationships (id, source_entity, target_entity, relationship_type, created_at, last_seen, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                last_seen = excluded.last_seen,
                metadata = excluded.metadata",
            params![
                id,
                rel.source_entity,
                rel.target_entity,
                rel.relationship_type,
                rel.created_at_ms,
                rel.last_seen_ms,
                metadata_json,
            ],
        )?;
        Ok(id)
    }

    /// Query direct neighbors of an entity (1-hop).
    pub async fn query_neighbors(
        &self,
        entity_id: &str,
    ) -> Result<Vec<(Entity, Relationship, String)>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT e.id, e.entity_type, e.name, e.path, e.created_at, e.updated_at, e.metadata,
                    r.id, r.source_entity, r.target_entity, r.relationship_type, r.created_at, r.last_seen, r.metadata,
                    CASE WHEN r.source_entity = ?1 THEN 'outgoing' ELSE 'incoming' END as direction
             FROM relationships r
             JOIN entities e ON e.id = CASE WHEN r.source_entity = ?1 THEN r.target_entity ELSE r.source_entity END
             WHERE r.source_entity = ?1 OR r.target_entity = ?1
             ORDER BY r.last_seen DESC",
        )?;
        let rows = stmt.query_map(params![entity_id], |row| {
            let e_metadata_str: String = row.get(6)?;
            let e_metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&e_metadata_str).unwrap_or_default();
            let r_metadata_str: String = row.get(13)?;
            let r_metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&r_metadata_str).unwrap_or_default();
            Ok((
                Entity {
                    id: row.get(0)?,
                    entity_type: row.get(1)?,
                    name: row.get(2)?,
                    path: row.get(3)?,
                    created_at_ms: row.get(4)?,
                    updated_at_ms: row.get(5)?,
                    metadata: e_metadata,
                },
                Relationship {
                    id: row.get(7)?,
                    source_entity: row.get(8)?,
                    target_entity: row.get(9)?,
                    relationship_type: row.get(10)?,
                    created_at_ms: row.get(11)?,
                    last_seen_ms: row.get(12)?,
                    metadata: r_metadata,
                },
                row.get(14)?,
            ))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Build a subgraph rooted at `entity_id`, expanding up to `max_depth` hops.
    /// Uses BFS to collect all reachable entities and edges.
    pub async fn build_subgraph(&self, entity_id: &str, max_depth: usize) -> Result<Subgraph> {
        if max_depth == 0 {
            // Just return the root entity.
            let conn = self.db.conn().await;
            let mut stmt = conn.prepare(
                "SELECT id, entity_type, name, path, created_at, updated_at, metadata
                 FROM entities WHERE id = ?1",
            )?;
            let rows = stmt.query_map(params![entity_id], |row| {
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
            let nodes: Vec<Entity> = rows.filter_map(|r| r.ok()).collect();
            return Ok(Subgraph {
                nodes,
                edges: vec![],
                root_entity_id: entity_id.to_string(),
            });
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut nodes: Vec<Entity> = Vec::new();
        let mut edges: Vec<Relationship> = Vec::new();
        let mut frontier: Vec<String> = vec![entity_id.to_string()];
        visited.insert(entity_id.to_string());

        for _depth in 0..max_depth {
            let mut next_frontier: Vec<String> = Vec::new();
            for node_id in &frontier {
                let neighbors = self.query_neighbors(node_id).await?;
                for (entity, rel, _direction) in neighbors {
                    // Add edge.
                    edges.push(rel);
                    // Add entity if not visited.
                    if !visited.contains(&entity.id) {
                        visited.insert(entity.id.clone());
                        next_frontier.push(entity.id.clone());
                        nodes.push(entity);
                    }
                }
            }
            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
        }

        // Include the root entity in nodes.
        let root = self.get_entity_raw(entity_id).await?;
        if let Some(root) = root {
            nodes.insert(0, root);
        }

        // Deduplicate edges.
        let mut seen_edges: HashSet<String> = HashSet::new();
        edges.retain(|e| seen_edges.insert(e.id.clone()));

        Ok(Subgraph {
            nodes,
            edges,
            root_entity_id: entity_id.to_string(),
        })
    }

    /// Get all relationships of a specific type.
    pub async fn get_relationships_by_type(&self, rel_type: &str) -> Result<Vec<Relationship>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, source_entity, target_entity, relationship_type, created_at, last_seen, metadata
             FROM relationships WHERE relationship_type = ?1
             ORDER BY last_seen DESC",
        )?;
        let rows = stmt.query_map(params![rel_type], |row| {
            let metadata_str: String = row.get(6)?;
            let metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&metadata_str).unwrap_or_default();
            Ok(Relationship {
                id: row.get(0)?,
                source_entity: row.get(1)?,
                target_entity: row.get(2)?,
                relationship_type: row.get(3)?,
                created_at_ms: row.get(4)?,
                last_seen_ms: row.get(5)?,
                metadata,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Count relationships by type.
    pub async fn count_by_type(&self) -> Result<HashMap<String, i64>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT relationship_type, COUNT(*) FROM relationships GROUP BY relationship_type",
        )?;
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

    /// Remove stale relationships not seen since `cutoff_ms`.
    pub async fn prune_stale(&self, cutoff_ms: i64) -> Result<usize> {
        let conn = self.db.conn().await;
        let deleted = conn.execute(
            "DELETE FROM relationships WHERE last_seen < ?1",
            params![cutoff_ms],
        )?;
        Ok(deleted)
    }

    async fn get_entity_raw(&self, id: &str) -> Result<Option<Entity>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twin::database::entity_store::EntityStore;
    use crate::twin::database::TwinDb;

    async fn setup() -> (TwinDb, EntityStore, GraphStore) {
        let db = TwinDb::open_memory().unwrap();
        let es = EntityStore::new(db.handle());
        let gs = GraphStore::new(db.handle());
        (db, es, gs)
    }

    fn make_entity(id: &str, et: &str, name: &str) -> Entity {
        Entity {
            id: id.to_string(),
            entity_type: et.to_string(),
            name: name.to_string(),
            path: None,
            created_at_ms: 1000,
            updated_at_ms: 1000,
            metadata: HashMap::new(),
        }
    }

    fn make_rel(src: &str, tgt: &str, rt: &str) -> Relationship {
        Relationship {
            id: String::new(),
            source_entity: src.to_string(),
            target_entity: tgt.to_string(),
            relationship_type: rt.to_string(),
            created_at_ms: 1000,
            last_seen_ms: 2000,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_add_and_query_neighbors() {
        let (_db, es, gs) = setup().await;
        es.create_entity(make_entity("app:xcode", "application", "Xcode"))
            .await
            .unwrap();
        es.create_entity(make_entity("file:main.swift", "file", "main.swift"))
            .await
            .unwrap();
        gs.add_relationship(make_rel("app:xcode", "file:main.swift", "created"))
            .await
            .unwrap();

        let neighbors = gs.query_neighbors("app:xcode").await.unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].0.name, "main.swift");
        assert_eq!(neighbors[0].2, "outgoing");
    }

    #[tokio::test]
    async fn test_build_subgraph_2hop() {
        let (_db, es, gs) = setup().await;
        es.create_entity(make_entity("a", "app", "A"))
            .await
            .unwrap();
        es.create_entity(make_entity("b", "file", "B"))
            .await
            .unwrap();
        es.create_entity(make_entity("c", "file", "C"))
            .await
            .unwrap();
        gs.add_relationship(make_rel("a", "b", "created"))
            .await
            .unwrap();
        gs.add_relationship(make_rel("b", "c", "depends_on"))
            .await
            .unwrap();

        let sg = gs.build_subgraph("a", 2).await.unwrap();
        assert!(sg.nodes.len() >= 3);
        assert!(sg.edges.len() >= 2);
    }

    #[tokio::test]
    async fn test_count_by_type() {
        let (_db, es, gs) = setup().await;
        es.create_entity(make_entity("a", "app", "A"))
            .await
            .unwrap();
        es.create_entity(make_entity("b", "file", "B"))
            .await
            .unwrap();
        es.create_entity(make_entity("c", "file", "C"))
            .await
            .unwrap();
        gs.add_relationship(make_rel("a", "b", "created"))
            .await
            .unwrap();
        gs.add_relationship(make_rel("a", "c", "created"))
            .await
            .unwrap();
        gs.add_relationship(make_rel("b", "c", "depends_on"))
            .await
            .unwrap();

        let counts = gs.count_by_type().await.unwrap();
        assert_eq!(counts.get("created"), Some(&2));
        assert_eq!(counts.get("depends_on"), Some(&1));
    }

    #[tokio::test]
    async fn test_prune_stale() {
        let (_db, es, gs) = setup().await;
        es.create_entity(make_entity("a", "app", "A"))
            .await
            .unwrap();
        es.create_entity(make_entity("b", "file", "B"))
            .await
            .unwrap();
        gs.add_relationship(make_rel("a", "b", "created"))
            .await
            .unwrap();

        let deleted = gs.prune_stale(3000).await.unwrap();
        assert_eq!(deleted, 1);
    }
}
