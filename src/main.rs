use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn};

mod cleanup;
mod cli;
mod core;
mod engines;
mod util;

use cli::{args::{Cli, OutputFormat}, output::OutputWriter};
use core::context::ScanContext;
use core::engine::Engine;
use core::types::Finding;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    init_tracing(cli.global.verbose);

    info!("X-MaC starting - engine: {:?}", cli.command.engine_id());

    let (tx, mut rx) = mpsc::channel::<Finding>(1000);

    let ctx = Arc::new(ScanContext::new(&cli, tx).await?);

    let output_writer = Arc::new(tokio::sync::Mutex::new(OutputWriter::new(&cli.global)));

    let collector_handle = {
        let output_writer = Arc::clone(&output_writer);
        tokio::spawn(async move {
            while let Some(finding) = rx.recv().await {
                let mut writer = output_writer.lock().await;
                writer.write_finding(&finding)?;
            }
            Ok::<_, anyhow::Error>(())
        })
    };

    let scan_start = Instant::now();

    let engine_results = match &cli.command {
        cli::args::Commands::Quick(args) => {
            run_quick(ctx.clone(), args).await
        }
        cli::args::Commands::Scan(args) | cli::args::Commands::Doctor(args) => {
            run_scan(ctx.clone(), args).await
        }
        cli::args::Commands::Clean(args) => {
            let engine = engines::clean::CleanEngine::new(args.clone());
            vec![engine.run(ctx.clone()).await]
        }
        cli::args::Commands::Conflict(args) => {
            let engine = engines::conflict::ConflictEngine::new(args.clone());
            vec![engine.run(ctx.clone()).await]
        }
        cli::args::Commands::Map(args) => {
            let engine = engines::map::MapEngine::new(args.clone());
            vec![engine.run(ctx.clone()).await]
        }
        cli::args::Commands::Envmap(args) => {
            let engine = engines::envmap::EnvmapEngine::new(args.clone());
            vec![engine.run(ctx.clone()).await]
        }
        cli::args::Commands::Depth(args) => {
            let engine = engines::depth::DepthEngine::new(args.clone());
            vec![engine.run(ctx.clone()).await]
        }
        cli::args::Commands::Maintain(args) => {
            let engine = engines::maintain::MaintainEngine::new(args.clone());
            vec![engine.run(ctx.clone()).await]
        }
        cli::args::Commands::Disk(args) => {
            let engine = engines::disk::DiskEngine::new(args.clone());
            vec![engine.run(ctx.clone()).await]
        }
        cli::args::Commands::Graph(args) => {
            let engine = engines::graph::GraphEngine::new(args.clone());
            vec![engine.run(ctx.clone()).await]
        }
        cli::args::Commands::All(args) => {
            run_all_engines(ctx.clone(), args).await
        }
        cli::args::Commands::Install(args) => {
            // Handle install before the scan pipeline — it doesn't scan.
            return run_install(&cli, args);
        }
        cli::args::Commands::Purge(args) => {
            return run_purge(&cli, args).await;
        }
        cli::args::Commands::RamBoost(args) => {
            return run_ram_boost(&cli, args.clone()).await;
        }
    };

    drop(ctx);

    collector_handle.await??;

    let total_duration = scan_start.elapsed();

    // Collect findings for the report and/or the fix-script generator.
    // `take_findings` drains the buffer, so we capture once and reuse.
    let collected_findings: Vec<Finding> = {
        let mut writer = output_writer.lock().await;
        writer.take_findings()
    };

    if cli.global.format == OutputFormat::Report {
        let mut writer = output_writer.lock().await;
        let engine_stats: Vec<core::types::EngineStats> = engine_results
            .iter()
            .filter_map(|r| r.as_ref().ok().cloned())
            .collect();

        let report = core::types::ScanReport::from_findings_and_stats(
            &collected_findings,
            &engine_stats,
            &util::macos::MacosUtils::get_macos_version(),
            util::macos::MacosUtils::is_apple_silicon(),
            total_duration,
        );
        writer.write_report(&report)?;
    } else {
        output_writer.lock().await.flush()?;
    }

    // Generate the remediation script if requested. This runs after the scan
    // report so the user can review findings first.
    if let Some(fix_script_path) = &cli.global.fix_script {
        let generator = cli::fix_script::FixScriptGenerator::new(fix_script_path.clone());
        match generator.write(&collected_findings) {
            Ok(path) => {
                if !cli.global.quiet {
                    eprintln!(
                        "Wrote remediation script to {} ({} findings). Review it, then run: bash {} --yes",
                        path.display(),
                        collected_findings.len(),
                        path.display()
                    );
                }
            }
            Err(e) => warn!("Failed to write fix script: {}", e),
        }
    }

    for result in engine_results {
        match result {
            Ok(stats) => info!("Engine completed: {:?}", stats),
            Err(e) => warn!("Engine error: {}", e),
        }
    }

    info!("X-MaC scan complete");
    Ok(())
}

fn init_tracing(verbosity: u8) {
    let filter = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

async fn run_all_engines(
    ctx: Arc<ScanContext>,
    args: &cli::args::AllArgs,
) -> Vec<std::result::Result<core::types::EngineStats, core::error::EngineError>> {
    use cli::args::EngineIdArg;

    let skip: Vec<EngineIdArg> = args.skip.clone();
    let should_run = |id: EngineIdArg| -> bool { !skip.contains(&id) };

    let mut results = Vec::new();

    if should_run(EngineIdArg::Clean) {
        let clean_engine = engines::clean::CleanEngine::default();
        results.push(clean_engine.run(ctx.clone()).await);
    }

    if should_run(EngineIdArg::Conflict) {
        let conflict_engine = engines::conflict::ConflictEngine::default();
        results.push(conflict_engine.run(ctx.clone()).await);
    }

    if should_run(EngineIdArg::Map) {
        let map_engine = engines::map::MapEngine::default();
        results.push(map_engine.run(ctx.clone()).await);
    }

    if should_run(EngineIdArg::Envmap) {
        let envmap_engine = engines::envmap::EnvmapEngine::default();
        results.push(envmap_engine.run(ctx.clone()).await);
    }

    if should_run(EngineIdArg::Depth) {
        let depth_engine = engines::depth::DepthEngine::default();
        results.push(depth_engine.run(ctx.clone()).await);
    }

    results
}

/// The `scan` command — the recommended default. Runs the safe, reliable
/// engines by default (clean, conflict, map) plus package-manager
/// diagnostics. The depth engine is opt-in via `--include-depth`.
async fn run_scan(
    ctx: Arc<ScanContext>,
    args: &cli::args::ScanArgs,
) -> Vec<std::result::Result<core::types::EngineStats, core::error::EngineError>> {
    use cli::args::ScanEngineIdArg;

    let skip: Vec<ScanEngineIdArg> = args.skip.clone();
    let should_run = |id: ScanEngineIdArg| -> bool { !skip.contains(&id) };

    let mut results = Vec::new();

    if should_run(ScanEngineIdArg::Clean) {
        let clean_engine = engines::clean::CleanEngine::default();
        results.push(clean_engine.run(ctx.clone()).await);
    }

    if should_run(ScanEngineIdArg::Conflict) {
        let conflict_engine = engines::conflict::ConflictEngine::default();
        results.push(conflict_engine.run(ctx.clone()).await);
    }

    if should_run(ScanEngineIdArg::Map) {
        let map_engine = engines::map::MapEngine::default();
        results.push(map_engine.run(ctx.clone()).await);
    }

    // envmap is read-only and safe — included in `scan` by default.
    if args.envmap && should_run(ScanEngineIdArg::Envmap) {
        let envmap_engine = engines::envmap::EnvmapEngine::default();
        results.push(envmap_engine.run(ctx.clone()).await);
    }

    // Depth is opt-in for `scan` — it can be noisy on large installations.
    // Only run if --include-depth is explicitly passed AND not skipped.
    if args.include_depth && should_run(ScanEngineIdArg::Depth) {
        let depth_engine = engines::depth::DepthEngine::default();
        results.push(depth_engine.run(ctx.clone()).await);
    }

    if args.diagnostics && should_run(ScanEngineIdArg::Diag) {
        let diag_engine = engines::diag::DiagEngine::default();
        results.push(diag_engine.run(ctx.clone()).await);
    }

    results
}

/// The `install` command — symlinks the built binary into a directory on
/// PATH so `xmac` can be run from anywhere.
fn run_install(cli: &Cli, args: &cli::args::InstallArgs) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Determine the install directory.
    let install_dir = match &args.dir {
        Some(d) => d.clone(),
        None => {
            if cfg!(target_os = "macos") {
                // Default: /opt/homebrew/bin on Apple Silicon, /usr/local/bin on Intel.
                if util::macos::MacosUtils::is_apple_silicon() {
                    std::path::PathBuf::from("/opt/homebrew/bin")
                } else {
                    std::path::PathBuf::from("/usr/local/bin")
                }
            } else {
                // Linux: /usr/local/bin is the standard user-installed binary location.
                std::path::PathBuf::from("/usr/local/bin")
            }
        }
    };

    if !install_dir.exists() {
        anyhow::bail!(
            "Install directory {} does not exist. Create it first or choose a different directory.",
            install_dir.display()
        );
    }

    // Find the built binary.
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Cannot determine current executable path: {}", e))?;

    let target = install_dir.join("xmac");

    if target.exists() || target.is_symlink() {
        if !args.force {
            anyhow::bail!(
                "{} already exists. Use --force to overwrite.",
                target.display()
            );
        }
        if let Err(e) = std::fs::remove_file(&target) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e.into());
            }
        }
    }

    std::os::unix::fs::symlink(&current_exe, &target)?;

    // Ensure the target is executable (the symlink resolves to the binary,
    // but set perms on the target path too for good measure).
    let _ = std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755));

    if !cli_quiet(cli) {
        eprintln!("Installed: {} -> {}", target.display(), current_exe.display());
        eprintln!("You can now run `xmac` from any directory.");
        eprintln!("If '{}' is not on your PATH, add it to your shell profile:", install_dir.display());
        eprintln!("  export PATH=\"{}:$PATH\"", install_dir.display());
    }

    Ok(())
}

/// Check if --quiet was passed (used by run_install which exits early).
fn cli_quiet(cli: &Cli) -> bool {
    cli.global.quiet
}

/// The `purge` command — runs a clean scan, builds a transactional plan, and
/// executes it with full safety checks and undo metadata.
async fn run_purge(cli: &Cli, args: &cli::args::PurgeArgs) -> Result<()> {
    use cli::args::PurgeCategoryArg;
    use cleanup::{CleanupExecutor, CleanupPolicy};

    let mut policy = CleanupPolicy::safe();
    if args.force_review {
        policy.allow_trash_overrides = true;
    }

    // 1. Run the clean scan to produce findings.
    let clean_args = cli::args::CleanArgs {
        min_age: args.min_age.clone(),
        min_size: args.min_size.clone(),
        ..engines::clean::CleanEngine::default_args()
    };

    let (tx, mut rx) = mpsc::channel::<Finding>(1000);
    let ctx = Arc::new(ScanContext::new(cli, tx).await?);
    let clean_engine = engines::clean::CleanEngine::new(clean_args);
    let _ = clean_engine.run(ctx.clone()).await;

    drop(ctx);
    let mut findings = Vec::new();
    while let Some(finding) = rx.recv().await {
        findings.push(finding);
    }

    // 2. Optionally filter to selected categories.
    if !args.category.is_empty() {
        let allowed: Vec<crate::core::types::Category> = args
            .category
            .iter()
            .map(|c| match c {
                PurgeCategoryArg::Cache => crate::core::types::Category::Cache,
                PurgeCategoryArg::TempFile => crate::core::types::Category::TempFile,
                PurgeCategoryArg::BuildArtifact => crate::core::types::Category::BuildArtifact,
                PurgeCategoryArg::PackageManagerCache => crate::core::types::Category::PackageManagerCache,
                PurgeCategoryArg::BrowserCache => crate::core::types::Category::BrowserCache,
                PurgeCategoryArg::Log => crate::core::types::Category::Log,
                PurgeCategoryArg::TrashBin => crate::core::types::Category::TrashBin,
                PurgeCategoryArg::XcodeArtifact => crate::core::types::Category::XcodeArtifact,
                PurgeCategoryArg::OrphanFile => crate::core::types::Category::OrphanFile,
                PurgeCategoryArg::LargeFile => crate::core::types::Category::LargeFile,
                PurgeCategoryArg::DuplicateFile => crate::core::types::Category::DuplicateFile,
                PurgeCategoryArg::MailAttachment => crate::core::types::Category::MailAttachment,
                PurgeCategoryArg::IosBackup => crate::core::types::Category::IosBackup,
                PurgeCategoryArg::LanguageFile => crate::core::types::Category::LanguageFile,
                PurgeCategoryArg::DocumentVersion => crate::core::types::Category::DocumentVersion,
                PurgeCategoryArg::UniversalBinary => crate::core::types::Category::UniversalBinary,
            })
            .collect();
        findings.retain(|f| allowed.contains(&f.category));
    }

    // 3. Build and execute the plan.
    let executor = CleanupExecutor::new(policy, args.dry_run);
    let plan = executor.plan(&findings);
    let mut executor = executor;
    let transaction = executor.execute(&plan);

    // 4. Output the transaction record.
    if cli.global.format == OutputFormat::Json || cli.global.format == OutputFormat::JsonPretty {
        let json = match cli.global.format {
            OutputFormat::JsonPretty => serde_json::to_string_pretty(&transaction)?,
            _ => serde_json::to_string(&transaction)?,
        };
        println!("{}", json);
    } else {
        let mode = if args.dry_run { "DRY RUN" } else { "LIVE" };
        eprintln!("X-MaC purge ({}) — transaction {}", mode, transaction.id);
        eprintln!("  Candidates:  {}", plan.candidates.len());
        eprintln!("  Executable:  {}", plan.executable().len());
        eprintln!("  Blocked:     {}", plan.blocked().len());
        eprintln!(
            "  Reclaimable: {}",
            crate::util::disk::format_bytes(plan.total_reclaimable_bytes())
        );
        eprintln!("  Succeeded:   {}", transaction.successful_count());
        eprintln!(
            "  Reclaimed:   {}",
            crate::util::disk::format_bytes(transaction.successful_bytes())
        );
        for action in &transaction.actions {
            let status = if action.success { "OK" } else { "SKIP" };
            eprintln!(
                "  [{}] {} -> {}",
                status,
                action.original_path.display(),
                action
                    .trashed_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "-".to_string())
            );
            if let Some(err) = &action.error {
                eprintln!("       error: {}", err);
            }
        }
    }

    Ok(())
}

/// The `quick` command — one-shot cleanup scan + maintenance + disk breakdown.
/// Like CleanMyMac's "Smart Scan": gives you a quick overview of system health
/// and reclaimable space, runs safe maintenance, and shows disk usage.
async fn run_quick(
    ctx: Arc<ScanContext>,
    args: &cli::args::QuickArgs,
) -> Vec<std::result::Result<core::types::EngineStats, core::error::EngineError>> {
    let mut results = Vec::new();

    // 1. Clean scan (with dedup if requested)
    let clean_args = cli::args::CleanArgs {
        dedup: args.dedup,
        ..engines::clean::CleanEngine::default_args()
    };
    let clean_engine = engines::clean::CleanEngine::new(clean_args);
    results.push(clean_engine.run(ctx.clone()).await);

    // 2. Maintenance tasks (safe ones only — no sudo)
    if !args.no_maintain {
        let maintain_engine = engines::maintain::MaintainEngine::default();
        results.push(maintain_engine.run(ctx.clone()).await);
    }

    // 3. Disk usage breakdown
    if !args.no_disk {
        let disk_args = cli::args::DiskArgs {
            top: 20,
            min_size: "100M".to_string(),
            paths: args.paths.clone(),
        };
        let disk_engine = engines::disk::DiskEngine::new(disk_args);
        results.push(disk_engine.run(ctx.clone()).await);
    }

    results
}

/// The `ram-boost` command — memory optimizer with before/after comparison.
async fn run_ram_boost(
    cli: &Cli,
    args: cli::args::RamBoostArgs,
) -> Result<()> {
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::channel::<core::types::Finding>(1000);
    let ctx = Arc::new(ScanContext::new(cli, tx).await?);

    // Run the RAM boost pipeline
    let boost_handle = {
        let ctx = ctx.clone();
        tokio::spawn(async move {
            engines::maintain::MaintainEngine::run_ram_boost(args, ctx).await
        })
    };

    drop(ctx);

    // Print findings as they arrive — JSON or plain text depending on --format
    let is_json = cli.global.format == cli::args::OutputFormat::Json
        || cli.global.format == cli::args::OutputFormat::JsonPretty;

    let mut findings = Vec::new();
    while let Some(finding) = rx.recv().await {
        if is_json {
            // Stream JSON lines immediately so the GUI can parse them
            serde_json::to_writer(std::io::stdout(), &finding)?;
            println!();
        } else {
            println!("{}", finding.title);
            println!("{}", finding.description);
            if let Some(hint) = &finding.remediation_hint {
                println!("  → {}", hint);
            }
            println!();
        }
        findings.push(finding);
    }

    boost_handle.await??;

    Ok(())
}
