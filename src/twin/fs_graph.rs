use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Filesystem Intelligence Graph — a knowledge graph of every file on the Mac.
///
/// Covers: file index, ownership, creators, app relationships, dependencies,
/// cache relationships, config relationships, duplicate content, file
/// importance prediction, storage growth forecasting, abandoned/orphan
/// detection, and cleanup simulation.
///
/// Maps to Digital Twin operations 81-120.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemGraph {
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub ownership_map: HashMap<PathBuf, String>,
    pub creator_map: HashMap<PathBuf, String>,
    pub dependency_edges: Vec<FileEdge>,
    pub cache_relationships: Vec<FileEdge>,
    pub duplicate_clusters: Vec<DuplicateCluster>,
    pub abandoned_files: Vec<PathBuf>,
    pub orphan_files: Vec<PathBuf>,
    pub storage_growth_trend: Option<f64>,
    pub exhaustion_forecast_days: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdge {
    pub source: PathBuf,
    pub target: PathBuf,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCluster {
    pub hash: String,
    pub files: Vec<PathBuf>,
    pub total_size_bytes: u64,
    pub recommended_keep: Option<PathBuf>,
}

impl FilesystemGraph {
    /// Collect the filesystem intelligence graph.
    pub fn collect() -> Self {
        // TODO: implement full filesystem graph (ops 81-120)
        Self {
            total_files: 0,
            total_size_bytes: 0,
            ownership_map: HashMap::new(),
            creator_map: HashMap::new(),
            dependency_edges: Vec::new(),
            cache_relationships: Vec::new(),
            duplicate_clusters: Vec::new(),
            abandoned_files: Vec::new(),
            orphan_files: Vec::new(),
            storage_growth_trend: None,
            exhaustion_forecast_days: None,
        }
    }
}
