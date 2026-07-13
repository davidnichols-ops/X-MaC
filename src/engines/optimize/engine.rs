use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::cli::args::OptimizeArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

use super::graph::GraphBuilder;
use super::telemetry::TelemetrySnapshot;

/// The optimize engine collects memory telemetry, builds a memory graph,
/// and (in future phases) runs GNN inference to predict pressure and
/// recommend actions. In Phase 1 (observe mode), it collects telemetry,
/// builds the graph, and emits findings with predictions.
pub struct OptimizeEngine {
    args: OptimizeArgs,
}

#[allow(dead_code)]
impl OptimizeEngine {
    pub fn new(args: OptimizeArgs) -> Self {
        Self { args }
    }

    /// Run the optimize engine — collect telemetry, build graph, emit findings.
    pub async fn run_optimize(
        args: OptimizeArgs,
        ctx: Arc<ScanContext>,
    ) -> Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut findings_count = 0u64;
        let mut errors_count = 0u64;
        // Collect telemetry snapshot
        let snapshot = TelemetrySnapshot::collect();

        // Build memory graph
        let mut builder =
            GraphBuilder::new(args.max_processes, !args.exclude_system, args.buffer_size);
        let graph = builder.build(&snapshot);

        // Write graph to file if requested
        if let Some(ref graph_path) = args.output_graph {
            let json = serde_json::to_string_pretty(&graph).unwrap_or_default();
            if let Err(e) = std::fs::write(graph_path, &json) {
                errors_count += 1;
                eprintln!(
                    "Warning: failed to write graph to {}: {}",
                    graph_path.display(),
                    e
                );
            }
        }

        // Emit system telemetry finding
        {
            let finding = Finding::new(
                EngineId::All,
                Severity::Info,
                Category::SystemInfo,
                Target::Process(0),
                "Memory telemetry snapshot".to_string(),
                format!(
                    "Collected memory graph: {} nodes, {} edges. \
                     Pressure: {}, utilization: {:.1}%, free: {}, compressed: {}, swap: {}",
                    graph.nodes.len(),
                    graph.edges.len(),
                    pressure_label(snapshot.system.pressure_level),
                    snapshot.system.utilization() * 100.0,
                    format_bytes(snapshot.system.free_bytes),
                    format_bytes(snapshot.system.compressor_bytes_used),
                    format_bytes(snapshot.system.swap_used_bytes),
                ),
            )
            .with_metadata(
                "graph".to_string(),
                serde_json::to_value(&graph).unwrap_or(serde_json::Value::Null),
            )
            .with_metadata(
                "system_telemetry".to_string(),
                serde_json::to_value(&snapshot.system).unwrap_or(serde_json::Value::Null),
            )
            .with_metadata(
                "process_count".to_string(),
                serde_json::Value::Number(snapshot.processes.len().into()),
            )
            .with_metadata(
                "predicted_pressure_60s".to_string(),
                graph
                    .predicted_pressure_60s
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null),
            )
            .with_metadata(
                "prediction_confidence".to_string(),
                graph
                    .prediction_confidence
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null),
            )
            .with_metadata(
                "time_to_critical_secs".to_string(),
                graph
                    .time_to_critical_secs
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null),
            );

            let _ = ctx.tx.send(finding).await;
            findings_count += 1;
        }

        // Emit pressure warning finding if predicted
        if let Some(pred) = graph.predicted_pressure_60s {
            if pred > 1.0 {
                let finding = Finding::new(
                    EngineId::All,
                    Severity::Medium,
                    Category::RamOptimization,
                    Target::Process(0),
                    "Memory pressure predicted".to_string(),
                    format!(
                        "Predicted pressure level {:.1} in 60s (confidence: {:.0}%). \
                         Current utilization: {:.1}%. Time to critical: {}",
                        pred,
                        graph.prediction_confidence.unwrap_or(0.0) * 100.0,
                        snapshot.system.utilization() * 100.0,
                        graph
                            .time_to_critical_secs
                            .map(|s| format!("{:.0}s", s))
                            .unwrap_or_else(|| "N/A".to_string()),
                    ),
                );
                let _ = ctx.tx.send(finding).await;
                findings_count += 1;
            }
        }

        // Emit top memory consumer findings
        let mut top_procs: Vec<_> = snapshot
            .processes
            .iter()
            .filter(|p| !p.is_system || !args.exclude_system)
            .collect();
        top_procs.sort_by_key(|b| std::cmp::Reverse(b.resident_size));
        for proc in top_procs.into_iter().take(args.top_n) {
            let finding = Finding::new(
                EngineId::All,
                if proc.resident_size > 1_000_000_000 {
                    Severity::Medium
                } else {
                    Severity::Info
                },
                Category::RamOptimization,
                Target::Process(proc.pid),
                format!("Process {} (pid {})", proc.name, proc.pid),
                format!(
                    "RSS: {}, virtual: {}, threads: {}, {}{}",
                    format_bytes(proc.resident_size),
                    format_bytes(proc.virtual_size),
                    proc.thread_count,
                    if proc.compressed_bytes > 0 {
                        format!("compressed: {}, ", format_bytes(proc.compressed_bytes))
                    } else {
                        String::new()
                    },
                    if proc.is_system { "[system]" } else { "[user]" },
                ),
            )
            .with_size(proc.resident_size)
            .with_metadata(
                "process_telemetry".to_string(),
                serde_json::to_value(proc).unwrap_or(serde_json::Value::Null),
            );
            let _ = ctx.tx.send(finding).await;
            findings_count += 1;
        }

        Ok(EngineStats {
            engine: EngineId::All,
            duration: start.elapsed(),
            items_scanned: snapshot.processes.len() as u64,
            findings_count,
            errors_count,
        })
    }
}

#[async_trait]
impl Engine for OptimizeEngine {
    fn id(&self) -> EngineId {
        EngineId::All
    }

    fn name(&self) -> &'static str {
        "Optimize Engine"
    }

    fn description(&self) -> &'static str {
        "Collects memory telemetry, builds a memory graph, and predicts pressure"
    }

    async fn validate(&self, _ctx: &ScanContext) -> Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> Result<EngineStats, EngineError> {
        Self::run_optimize(self.args.clone(), ctx).await
    }
}

fn pressure_label(level: u32) -> &'static str {
    match level {
        4 => "Critical",
        2 => "Warning",
        _ => "Nominal",
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, UNITS[unit])
}
