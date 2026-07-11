use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A node in the file system graph — represents a file or directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: u32,
    pub path: String,
    pub name: String,
    pub node_type: NodeType,
    pub size_bytes: u64,
    pub depth: usize,
    pub num_children: u32,
    pub modified_secs: u64,
    pub accessed_secs: u64,
    pub is_hidden: bool,
    pub extension: Option<String>,
    pub features: Vec<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    File,
    Directory,
    Symlink,
}

/// An edge in the file system graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: u32,
    pub target: u32,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    ParentChild,
    Symlink,
}

/// The complete extracted graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub root_path: String,
    pub num_features: usize,
    pub extracted_at: u64,
}

/// Graph extractor — walks the file system and builds a graph.
pub struct GraphExtractor {
    max_depth: usize,
    max_nodes: usize,
    include_hidden: bool,
}

impl GraphExtractor {
    pub fn new(max_depth: usize, max_nodes: usize, include_hidden: bool) -> Self {
        Self {
            max_depth,
            max_nodes,
            include_hidden,
        }
    }

    /// Extract a file system graph from the given root path.
    pub fn extract(&self, root: &Path) -> FileSystemGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut path_to_id: HashMap<PathBuf, u32> = HashMap::new();
        let mut next_id: u32 = 0;

        self.walk(
            root,
            0,
            &mut next_id,
            &mut nodes,
            &mut edges,
            &mut path_to_id,
        );

        // The walk is post-order so a parent is appended after its children.
        // Enforce the hard cap after traversal as a final safety boundary.
        if nodes.len() > self.max_nodes {
            nodes.truncate(self.max_nodes);
            edges.retain(|edge| {
                edge.source < self.max_nodes as u32 && edge.target < self.max_nodes as u32
            });
        }

        let num_features = if nodes.is_empty() {
            0
        } else {
            nodes[0].features.len()
        };

        let extracted_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        FileSystemGraph {
            nodes,
            edges,
            root_path: root.display().to_string(),
            num_features,
            extracted_at,
        }
    }

    fn walk(
        &self,
        path: &Path,
        depth: usize,
        next_id: &mut u32,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<GraphEdge>,
        path_to_id: &mut HashMap<PathBuf, u32>,
    ) {
        if nodes.len() >= self.max_nodes {
            return;
        }
        if depth > self.max_depth {
            return;
        }

        let meta = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(_) => return,
        };

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        let is_hidden = name.starts_with('.');

        if !self.include_hidden && is_hidden && depth > 0 {
            return;
        }

        let node_type = if meta.file_type().is_symlink() {
            NodeType::Symlink
        } else if meta.file_type().is_dir() {
            NodeType::Directory
        } else {
            NodeType::File
        };

        let size_bytes = if node_type == NodeType::File {
            meta.len()
        } else {
            0
        };

        let modified_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let accessed_secs = meta
            .accessed()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let extension = path.extension().map(|e| e.to_string_lossy().to_string());

        let node_id = *next_id;
        *next_id += 1;

        path_to_id.insert(path.to_path_buf(), node_id);

        // Build feature vector:
        // [log_size, depth_norm, is_dir, is_file, is_symlink, is_hidden,
        //  age_days_norm, access_age_days_norm, has_extension]
        let features = vec![
            if size_bytes > 0 {
                (size_bytes as f32).ln() / 30.0
            } else {
                0.0
            },
            depth as f32 / self.max_depth as f32,
            if node_type == NodeType::Directory { 1.0 } else { 0.0 },
            if node_type == NodeType::File { 1.0 } else { 0.0 },
            if node_type == NodeType::Symlink { 1.0 } else { 0.0 },
            if is_hidden { 1.0 } else { 0.0 },
            {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if modified_secs > 0 && now > modified_secs {
                    ((now - modified_secs) / 86400) as f32 / 365.0
                } else {
                    0.0
                }
            },
            {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if accessed_secs > 0 && now > accessed_secs {
                    ((now - accessed_secs) / 86400) as f32 / 365.0
                } else {
                    0.0
                }
            },
            if extension.is_some() { 1.0 } else { 0.0 },
        ];

        let mut node = GraphNode {
            id: node_id,
            path: path.display().to_string(),
            name,
            node_type,
            size_bytes,
            depth,
            num_children: 0,
            modified_secs,
            accessed_secs,
            is_hidden,
            extension,
            features,
        };

        // If directory, walk children and create edges
        if node_type == NodeType::Directory {
            let mut children_count = 0u32;
            if let Ok(entries) = std::fs::read_dir(path) {
                let mut child_paths: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .collect();
                // Sort for deterministic output
                child_paths.sort();

                for child in child_paths {
                    if nodes.len() >= self.max_nodes {
                        break;
                    }

                    let parent_id = node_id;
                    self.walk(
                        &child,
                        depth + 1,
                        next_id,
                        nodes,
                        edges,
                        path_to_id,
                    );

                    if let Some(&child_id) = path_to_id.get(&child) {
                        edges.push(GraphEdge {
                            source: parent_id,
                            target: child_id,
                            edge_type: EdgeType::ParentChild,
                        });
                        children_count += 1;
                    }
                }
            }
            node.num_children = children_count;
        }

        nodes.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn extracts_nodes_and_parent_child_edges() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("cache");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("artifact.bin"), b"data").unwrap();

        let graph = GraphExtractor::new(4, 100, true).extract(temp.path());

        assert!(graph.nodes.iter().any(|node| node.name == "cache"));
        assert!(graph.nodes.iter().any(|node| node.name == "artifact.bin"));
        assert!(graph.edges.iter().any(|edge| edge.edge_type == EdgeType::ParentChild));
        assert_eq!(graph.num_features, 9);
    }

    #[test]
    fn respects_node_limit() {
        let temp = TempDir::new().unwrap();
        for index in 0..10 {
            std::fs::write(temp.path().join(format!("file-{index}")), b"data").unwrap();
        }

        let graph = GraphExtractor::new(2, 3, true).extract(temp.path());
        assert!(graph.nodes.len() <= 3);
    }
}
