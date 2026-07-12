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
            false,
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
        parent_is_dir: bool,
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
        let normalized_extension = extension.as_deref().map(str::to_ascii_lowercase);
        let is_executable = is_executable(&meta, normalized_extension.as_deref());

        let node_id = *next_id;
        *next_id += 1;

        path_to_id.insert(path.to_path_buf(), node_id);

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
            features: Vec::new(),
        };

        // If directory, walk children and create edges
        if node_type == NodeType::Directory {
            let mut children_count = 0u32;
            if let Ok(entries) = std::fs::read_dir(path) {
                let mut child_paths: Vec<PathBuf> =
                    entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
                // Sort for deterministic output
                child_paths.sort();

                for child in child_paths {
                    if nodes.len() >= self.max_nodes {
                        break;
                    }

                    let parent_id = node_id;
                    self.walk(&child, depth + 1, true, next_id, nodes, edges, path_to_id);

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

        // Build the 16-feature vector after walking so num_children is final.
        node.features = build_features(
            size_bytes,
            depth,
            self.max_depth,
            node_type,
            is_hidden,
            modified_secs,
            accessed_secs,
            normalized_extension.as_deref(),
            node.num_children,
            is_executable,
            parent_is_dir,
        );

        nodes.push(node);
    }
}

fn build_features(
    size_bytes: u64,
    depth: usize,
    max_depth: usize,
    node_type: NodeType,
    is_hidden: bool,
    modified_secs: u64,
    accessed_secs: u64,
    extension: Option<&str>,
    num_children: u32,
    is_executable: bool,
    parent_is_dir: bool,
) -> Vec<f32> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let normalized_age = |timestamp: u64| {
        if timestamp == 0 {
            0.0
        } else {
            (now.saturating_sub(timestamp) as f32 / 86_400.0 / 365.0).clamp(0.0, 1.0)
        }
    };
    let extension_in = |group: &[&str]| extension.is_some_and(|ext| group.contains(&ext));
    let binary = |value: bool| if value { 1.0 } else { 0.0 };

    vec![
        ((size_bytes as f64 + 1.0).ln() as f32 / 30.0).clamp(0.0, 1.0),
        if max_depth == 0 {
            0.0
        } else {
            (depth as f32 / max_depth as f32).clamp(0.0, 1.0)
        },
        binary(node_type == NodeType::Directory),
        binary(node_type == NodeType::File),
        binary(node_type == NodeType::Symlink),
        binary(is_hidden),
        normalized_age(modified_secs),
        normalized_age(accessed_secs),
        binary(extension.is_some()),
        (num_children as f32 / 50.0).clamp(0.0, 1.0),
        binary(is_executable),
        binary(extension_in(&[
            "zip", "gz", "tar", "dmg", "iso", "rar", "7z",
        ])),
        binary(extension_in(&[
            "rs", "swift", "py", "js", "ts", "c", "cpp", "h", "go", "rb", "java",
        ])),
        binary(extension_in(&[
            "png", "jpg", "jpeg", "mp4", "mov", "mp3", "aac", "pdf",
        ])),
        binary(extension_in(&[
            "json", "yaml", "yml", "toml", "xml", "plist", "conf", "ini", "env",
        ])),
        binary(parent_is_dir),
    ]
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata, extension: Option<&str>) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0 || extension == Some("app")
}

#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata, extension: Option<&str>) -> bool {
    extension == Some("app")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn assert_features(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), 16);
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() < 0.000_001,
                "feature {index}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn emits_all_sixteen_features_in_order() {
        let file_features = build_features(
            0,
            0,
            0,
            NodeType::File,
            false,
            0,
            0,
            Some("rs"),
            0,
            false,
            false,
        );
        assert_features(
            &file_features,
            &[
                0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
            ],
        );

        let saturated_features = build_features(
            u64::MAX,
            10,
            2,
            NodeType::Directory,
            true,
            1,
            1,
            Some("zip"),
            75,
            true,
            true,
        );
        assert_features(
            &saturated_features,
            &[
                1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            ],
        );

        let symlink_features = build_features(
            0,
            1,
            2,
            NodeType::Symlink,
            false,
            u64::MAX,
            u64::MAX,
            Some("mp4"),
            0,
            false,
            true,
        );
        assert_features(
            &symlink_features,
            &[
                0.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0,
            ],
        );

        let config_features = build_features(
            0,
            0,
            1,
            NodeType::File,
            false,
            0,
            0,
            Some("yaml"),
            0,
            false,
            true,
        );
        assert_eq!(config_features[14], 1.0);
        assert!(config_features
            .iter()
            .all(|value| (0.0..=1.0).contains(value)));
    }

    #[test]
    fn extraction_sets_children_parent_extension_and_app_features() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("archive.ZIP"), b"data").unwrap();
        std::fs::write(temp.path().join("source.RS"), b"data").unwrap();
        std::fs::write(temp.path().join("media.MP4"), b"data").unwrap();
        std::fs::write(temp.path().join("config.YAML"), b"data").unwrap();
        std::fs::create_dir(temp.path().join("Tool.APP")).unwrap();

        let graph = GraphExtractor::new(2, 100, true).extract(temp.path());
        assert_eq!(graph.num_features, 16);
        assert!(graph.nodes.iter().all(|node| node.features.len() == 16));

        let root = graph.nodes.iter().find(|node| node.depth == 0).unwrap();
        assert_eq!(root.num_children, 5);
        assert_eq!(root.features[9], 0.1);
        assert_eq!(root.features[15], 0.0);

        let feature_for = |name: &str, index: usize| {
            graph
                .nodes
                .iter()
                .find(|node| node.name == name)
                .unwrap()
                .features[index]
        };
        assert_eq!(feature_for("archive.ZIP", 11), 1.0);
        assert_eq!(feature_for("source.RS", 12), 1.0);
        assert_eq!(feature_for("media.MP4", 13), 1.0);
        assert_eq!(feature_for("config.YAML", 14), 1.0);
        assert_eq!(feature_for("Tool.APP", 10), 1.0);
        assert_eq!(feature_for("Tool.APP", 15), 1.0);
    }

    #[cfg(unix)]
    #[test]
    fn detects_unix_executable_mode() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let executable = temp.path().join("command");
        std::fs::write(&executable, b"data").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();

        let graph = GraphExtractor::new(1, 100, true).extract(temp.path());
        let node = graph
            .nodes
            .iter()
            .find(|node| node.name == "command")
            .unwrap();
        assert_eq!(node.features[10], 1.0);
    }

    #[test]
    fn extracts_nodes_and_parent_child_edges() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("cache");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("artifact.bin"), b"data").unwrap();

        let graph = GraphExtractor::new(4, 100, true).extract(temp.path());

        assert!(graph.nodes.iter().any(|node| node.name == "cache"));
        assert!(graph.nodes.iter().any(|node| node.name == "artifact.bin"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.edge_type == EdgeType::ParentChild));
        assert_eq!(graph.num_features, 16);
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
