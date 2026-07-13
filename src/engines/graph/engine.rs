use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::cli::args::GraphArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

use super::extractor::GraphExtractor;

/// The graph engine extracts a file system graph for GNN consumption.
/// It walks the file system and emits nodes (files/dirs with features) and
/// edges (parent-child, symlink) as JSON. The graph can be written to disk
/// via --output-graph or embedded as metadata in findings.
pub struct GraphEngine {
    args: GraphArgs,
}

impl GraphEngine {
    pub fn new(args: GraphArgs) -> Self {
        Self { args }
    }
}

#[async_trait]
impl Engine for GraphEngine {
    fn id(&self) -> EngineId {
        EngineId::All
    }

    fn name(&self) -> &'static str {
        "Graph Engine"
    }

    fn description(&self) -> &'static str {
        "Extracts a file system graph (nodes + edges) for GNN training and inference"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();

        let roots: Vec<std::path::PathBuf> = if self.args.paths.is_empty() {
            vec![std::env::var("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))]
        } else {
            self.args.paths.clone()
        };

        let include_hidden = ctx.config.include_hidden;

        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        for root in &roots {
            let graph = tokio::task::spawn_blocking({
                let root = root.clone();
                let extractor =
                    GraphExtractor::new(self.args.max_depth, self.args.max_nodes, include_hidden);
                move || extractor.extract(&root)
            })
            .await
            .map_err(|e| EngineError::ScanFailed(e.to_string()))?;

            items_scanned += graph.nodes.len() as u64;

            // If --output-graph was specified, write graph JSON files
            if let Some(output_dir) = &self.args.output_graph {
                if let Err(e) = std::fs::create_dir_all(output_dir) {
                    let finding = Finding::new(
                        EngineId::All,
                        Severity::Medium,
                        Category::SystemInfo,
                        Target::Path(output_dir.clone()),
                        "Graph output directory creation failed",
                        format!("Failed to create output directory: {}", e),
                    );
                    ctx.emit(finding).await;
                    findings_count += 1;
                    continue;
                }

                let graph_json = serde_json::to_string(&graph)
                    .map_err(|e| EngineError::ScanFailed(e.to_string()))?;

                let safe_name = root
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "root".to_string())
                    .replace('/', "_");

                let output_file = output_dir.join(format!("graph_{}.json", safe_name));
                if let Err(e) = std::fs::write(&output_file, graph_json) {
                    let finding = Finding::new(
                        EngineId::All,
                        Severity::Medium,
                        Category::SystemInfo,
                        Target::Path(output_file.clone()),
                        "Failed to write graph",
                        format!("Failed to write graph to {}: {}", output_file.display(), e),
                    );
                    ctx.emit(finding).await;
                } else {
                    let finding = Finding::new(
                        EngineId::All,
                        Severity::Info,
                        Category::SystemInfo,
                        Target::Path(output_file.clone()),
                        format!("File system graph: {} ({} nodes, {} edges)", safe_name, graph.nodes.len(), graph.edges.len()),
                        format!(
                            "Extracted graph from {} — {} nodes, {} edges, {} features per node. Written to {}",
                            graph.root_path,
                            graph.nodes.len(),
                            graph.edges.len(),
                            graph.num_features,
                            output_file.display(),
                        ),
                    )
                    .with_hint(format!("cat {}", output_file.display()));
                    ctx.emit(finding).await;
                }
            } else {
                // Embed graph as metadata in a finding
                let graph_json = serde_json::to_value(&graph)
                    .map_err(|e| EngineError::ScanFailed(e.to_string()))?;

                let finding = Finding::new(
                    EngineId::All,
                    Severity::Info,
                    Category::SystemInfo,
                    Target::Path(root.clone()),
                    format!(
                        "File system graph: {} ({} nodes, {} edges)",
                        root.display(),
                        graph.nodes.len(),
                        graph.edges.len()
                    ),
                    format!(
                        "Extracted graph from {} — {} nodes, {} edges, {} features per node",
                        graph.root_path,
                        graph.nodes.len(),
                        graph.edges.len(),
                        graph.num_features,
                    ),
                )
                .with_metadata(
                    "graph_nodes",
                    serde_json::Value::Number(graph.nodes.len().into()),
                )
                .with_metadata(
                    "graph_edges",
                    serde_json::Value::Number(graph.edges.len().into()),
                )
                .with_metadata(
                    "graph_max_depth",
                    serde_json::Value::Number(self.args.max_depth.into()),
                )
                .with_metadata(
                    "num_features",
                    serde_json::Value::Number(graph.num_features.into()),
                );

                let finding = if graph.nodes.len() <= 5000 {
                    // Include full graph for small graphs
                    finding.with_metadata("graph", graph_json)
                } else {
                    // For large graphs, just include summary stats
                    finding
                };

                ctx.emit(finding).await;
            }

            findings_count += 1;
        }

        Ok(EngineStats {
            engine: self.id(),
            duration: start.elapsed(),
            items_scanned,
            findings_count,
            errors_count: 0,
        })
    }
}
