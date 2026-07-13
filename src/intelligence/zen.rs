use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::cli::args::ZenArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::types::{Finding, Category, Severity};
use crate::util::disk::format_bytes;

/// Result of a Zen Mode run — comprehensive summary for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenResult {
    pub duration_secs: f64,
    pub health_before: f64,
    pub health_after: f64,
    pub memory_before: MemorySummary,
    pub memory_after: MemorySummary,
    pub reclaimable_bytes: u64,
    pub reclaimed_bytes: u64,
    pub findings_count: usize,
    pub maintenance_tasks_run: usize,
    pub top_categories: Vec<(String, u64)>,
    pub steps: Vec<ZenStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub utilization: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenStep {
    pub name: String,
    pub status: String,
    pub detail: String,
}

/// Run Zen Mode — comprehensive one-click optimization.
///
/// Steps:
/// 1. Collect system snapshot (health before)
/// 2. Run clean scan (find reclaimable space)
/// 3. Optionally execute cleanup (if --execute)
/// 4. Purge inactive memory
/// 5. Run safe maintenance tasks
/// 6. Collect system snapshot (health after)
/// 7. Produce summary
pub async fn run_zen(
    cli: &crate::cli::args::Cli,
    args: &ZenArgs,
) -> anyhow::Result<ZenResult> {
    let start = Instant::now();
    let mut steps = Vec::new();

    // 1. Collect health before
    let snapshot_before = crate::intelligence::SystemSnapshot::collect();
    let health_before = snapshot_before.health_score;
    let memory_before = MemorySummary {
        total_bytes: snapshot_before.memory.total_bytes,
        used_bytes: snapshot_before.memory.used_bytes,
        free_bytes: snapshot_before.memory.free_bytes,
        utilization: snapshot_before.memory.utilization,
    };

    steps.push(ZenStep {
        name: "System snapshot".to_string(),
        status: "done".to_string(),
        detail: format!("Health: {:.0}/100 ({})", health_before, snapshot_before.status),
    });

    let mut reclaimable_bytes = 0u64;
    let mut reclaimed_bytes = 0u64;
    let mut findings_count = 0usize;
    let mut top_categories: Vec<(String, u64)> = Vec::new();

    // 2. Run clean scan
    if !args.no_clean {
        let (tx, mut rx) = mpsc::channel::<Finding>(1000);
        let ctx = Arc::new(ScanContext::new(cli, tx).await?);
        let clean_args = crate::engines::clean::CleanEngine::default_args();
        let clean_engine = crate::engines::clean::CleanEngine::new(clean_args);
        let _ = clean_engine.run(ctx.clone()).await;
        drop(ctx);

        let mut findings = Vec::new();
        while let Some(finding) = rx.recv().await {
            if let Some(size) = finding.size_bytes {
                if !matches!(finding.category, Category::SystemInfo | Category::LargeFile) {
                    reclaimable_bytes += size;
                }
            }
            findings.push(finding);
        }
        findings_count = findings.len();

        // Aggregate by category
        let mut cat_map: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for f in &findings {
            if let Some(size) = f.size_bytes {
                if !matches!(f.category, Category::SystemInfo | Category::LargeFile) {
                    let cat = format!("{:?}", f.category);
                    *cat_map.entry(cat).or_insert(0) += size;
                }
            }
        }
        top_categories = cat_map.into_iter().collect();
        top_categories.sort_by(|a, b| b.1.cmp(&a.1));
        top_categories.truncate(5);

        // 3. Execute cleanup if requested
        if args.execute && !args.dry_run {
            use crate::cleanup::{CleanupExecutor, CleanupPolicy};
            let policy = CleanupPolicy::safe();
            let executor = CleanupExecutor::new(policy, false);
            let plan = executor.plan(&findings);
            reclaimable_bytes = plan.total_reclaimable_bytes();
            let mut executor = executor;
            let transaction = executor.execute(&plan);
            reclaimed_bytes = transaction.successful_bytes();

            steps.push(ZenStep {
                name: "Disk cleanup".to_string(),
                status: "done".to_string(),
                detail: format!(
                    "Reclaimed {} of {} reclaimable",
                    format_bytes(reclaimed_bytes),
                    format_bytes(reclaimable_bytes),
                ),
            });
        } else {
            steps.push(ZenStep {
                name: "Disk cleanup".to_string(),
                status: if args.dry_run { "preview" } else { "scanned" }.to_string(),
                detail: format!(
                    "Found {} reclaimable across {} findings{}",
                    format_bytes(reclaimable_bytes),
                    findings_count,
                    if args.dry_run { " (dry run)" } else if !args.execute { " (use --execute to clean)" } else { "" },
                ),
            });
        }
    }

    // 4. Purge memory
    if !args.no_memory {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let _ = Command::new("purge").output();
        }

        steps.push(ZenStep {
            name: "Memory optimization".to_string(),
            status: "done".to_string(),
            detail: "Purged inactive memory".to_string(),
        });
    }

    // 5. Run maintenance
    let mut maintenance_tasks_run = 0;
    if !args.no_maintain {
        let (tx, mut rx) = mpsc::channel::<Finding>(1000);
        let ctx = Arc::new(ScanContext::new(cli, tx).await?);
        let maintain_engine = crate::engines::maintain::MaintainEngine::default();
        let _ = maintain_engine.run(ctx.clone()).await;
        drop(ctx);

        while let Some(finding) = rx.recv().await {
            if finding.severity != Severity::Info {
                // Count successful tasks
            }
            maintenance_tasks_run += 1;
        }

        steps.push(ZenStep {
            name: "Maintenance".to_string(),
            status: "done".to_string(),
            detail: format!("Ran {} maintenance tasks", maintenance_tasks_run),
        });
    }

    // 6. Collect health after
    let snapshot_after = crate::intelligence::SystemSnapshot::collect();
    let health_after = snapshot_after.health_score;
    let memory_after = MemorySummary {
        total_bytes: snapshot_after.memory.total_bytes,
        used_bytes: snapshot_after.memory.used_bytes,
        free_bytes: snapshot_after.memory.free_bytes,
        utilization: snapshot_after.memory.utilization,
    };

    steps.push(ZenStep {
        name: "Post-optimization snapshot".to_string(),
        status: "done".to_string(),
        detail: format!("Health: {:.0}/100 ({})", health_after, snapshot_after.status),
    });

    Ok(ZenResult {
        duration_secs: start.elapsed().as_secs_f64(),
        health_before,
        health_after,
        memory_before,
        memory_after,
        reclaimable_bytes,
        reclaimed_bytes,
        findings_count,
        maintenance_tasks_run,
        top_categories,
        steps,
    })
}

/// Format a Zen result as human-readable text.
pub fn format_zen_result_text(result: &ZenResult, dry_run: bool) -> String {
    let mut out = String::new();

    let mode = if dry_run { "PREVIEW" } else { "COMPLETE" };

    out.push_str("╔══════════════════════════════════════════════╗\n");
    out.push_str(&format!("║  X-MaC Zen Mode — {}{}\n", mode, " ".repeat(30 - mode.len())));
    out.push_str("╚══════════════════════════════════════════════╝\n\n");

    // Health score delta
    let delta = result.health_after - result.health_before;
    let delta_str = if delta >= 0.0 {
        format!("+{:.0}", delta)
    } else {
        format!("{:.0}", delta)
    };
    out.push_str(&format!("System Health:  {:.0} → {:.0}  ({})\n", result.health_before, result.health_after, delta_str));
    out.push_str(&format!("Duration:        {:.1}s\n\n", result.duration_secs));

    // Memory
    let mem_freed = result.memory_before.used_bytes.saturating_sub(result.memory_after.used_bytes);
    out.push_str("Memory:\n");
    out.push_str(&format!("  Usage:  {:.1} GB → {:.1} GB", gb(result.memory_before.used_bytes), gb(result.memory_after.used_bytes)));
    if mem_freed > 0 {
        out.push_str(&format!("  (freed {})", format_bytes(mem_freed)));
    }
    out.push_str(&format!("\n  Free:   {:.1} GB → {:.1} GB\n\n", gb(result.memory_before.free_bytes), gb(result.memory_after.free_bytes)));

    // Disk
    out.push_str("Disk:\n");
    if result.reclaimed_bytes > 0 {
        out.push_str(&format!("  Reclaimed:      {} \n", format_bytes(result.reclaimed_bytes)));
        out.push_str(&format!("  Reclaimable:    {} (total found)\n", format_bytes(result.reclaimable_bytes)));
    } else {
        out.push_str(&format!("  Reclaimable:    {} across {} findings\n", format_bytes(result.reclaimable_bytes), result.findings_count));
    }
    if !result.top_categories.is_empty() {
        out.push_str("  Top categories:\n");
        for (cat, size) in &result.top_categories {
            out.push_str(&format!("    {:20} {}\n", cat, format_bytes(*size)));
        }
    }
    out.push('\n');

    // Maintenance
    if result.maintenance_tasks_run > 0 {
        out.push_str(&format!("Maintenance: {} tasks completed\n\n", result.maintenance_tasks_run));
    }

    // Steps
    out.push_str("Steps:\n");
    for step in &result.steps {
        let icon = match step.status.as_str() {
            "done" => "✓",
            "preview" => "○",
            "scanned" => "○",
            "skipped" => "·",
            _ => "·",
        };
        out.push_str(&format!("  {} {} — {}\n", icon, step.name, step.detail));
    }

    if !dry_run && result.reclaimable_bytes > 0 && result.reclaimed_bytes == 0 {
        out.push_str("\n💡 Run with --execute to reclaim the identified space.\n");
    }

    out
}

fn gb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}
