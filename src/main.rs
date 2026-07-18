use anyhow::{Context, Result};
use clap::Parser;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn};

mod cleanup;
mod cli;
mod config;
mod core;
mod engines;
mod intelligence;
mod mcp;
mod safety;
mod twin;
mod util;

use cli::{
    args::{Cli, OutputFormat},
    output::OutputWriter,
};
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
            // Load safety engine once for enriching findings as they arrive.
            let safety_engine = safety::rule_engine::SafetyEngine::load_default().ok();
            while let Some(mut finding) = rx.recv().await {
                // Enrich with safety classification before output.
                if let Some(ref engine) = safety_engine {
                    if let core::types::Target::Path(ref path) = finding.target {
                        if let Some(classification) = engine.classify(&path.to_string_lossy()) {
                            finding.safety_rating = Some(classification.rating.label().to_string());
                            finding.safety_rule = Some(classification.rule_name.clone());
                            finding.safety_explanation = Some(classification.explanation());
                            finding.safety_confidence = Some(classification.confidence);
                        }
                    }
                }
                let mut writer = output_writer.lock().await;
                writer.write_finding(&finding)?;
            }
            Ok::<_, anyhow::Error>(())
        })
    };

    let scan_start = Instant::now();

    // Load config once — used to apply profile-based overrides to engines
    let xmac_config = config::ConfigManager::load();
    let xmac_config = xmac_config.config().clone();

    let engine_results = match &cli.command {
        cli::args::Commands::Quick(args) => run_quick(ctx.clone(), args).await,
        cli::args::Commands::Scan(args) | cli::args::Commands::Doctor(args) => {
            run_scan(ctx.clone(), args).await
        }
        cli::args::Commands::Clean(args) => {
            let engine = engines::clean::CleanEngine::new(args.clone()).with_config(&xmac_config);
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
            let engine =
                engines::maintain::MaintainEngine::new(args.clone()).with_config(&xmac_config);
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
        cli::args::Commands::All(args) => run_all_engines(ctx.clone(), args).await,
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
        cli::args::Commands::Optimize(args) => {
            return run_optimize(&cli, args.clone()).await;
        }
        cli::args::Commands::Config(args) => {
            return run_config(&cli, args);
        }
        cli::args::Commands::Daemon(args) => {
            return run_daemon(&cli, args.clone()).await;
        }
        cli::args::Commands::Zen(args) => {
            return run_zen(&cli, args).await;
        }
        cli::args::Commands::Advisor(args) => {
            return run_advisor(&cli, args).await;
        }
        cli::args::Commands::History(args) => {
            return run_history(&cli, args);
        }
        cli::args::Commands::Completions(args) => {
            return run_completions(args.clone());
        }
        cli::args::Commands::Twin(args) => {
            return run_twin(&cli, args.clone()).await;
        }
        cli::args::Commands::Mcp => {
            return mcp::run_server();
        }
        cli::args::Commands::Safety(args) => {
            return run_safety(&cli, args.clone());
        }
        cli::args::Commands::Dedup(args) => {
            let min_size = byte_unit::Byte::from_str(&args.min_size)
                .map(|b| b.get_bytes() as u64)
                .unwrap_or(1024);
            let engine = engines::duplicate::DuplicateEngine::new()
                .with_scan_paths(args.paths.clone())
                .with_min_size(min_size);
            vec![engine.run(ctx.clone()).await]
        }
    };

    drop(ctx);

    collector_handle.await??;

    let total_duration = scan_start.elapsed();

    // Collect findings for the report and/or the fix-script generator.
    // `take_findings` drains the buffer, so we capture once and reuse.
    // Note: safety enrichment happens in the collector task before output.
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

/// Type alias for a boxed engine future — avoids clippy type_complexity warnings.
type EngineFuture = std::pin::Pin<Box<dyn std::future::Future<Output = EngineResult> + Send>>;

/// Type alias for the result of running an engine.
type EngineResult = std::result::Result<core::types::EngineStats, core::error::EngineError>;

/// Determine the maximum number of engines to run concurrently based on
/// the resource mode. This controls top-level engine parallelism —
/// individual engines may have their own internal parallelism (e.g. the
/// clean engine's category-level FuturesUnordered).
///
/// - eco: 1 engine at a time (sequential — lowest CPU strain)
/// - balanced: 2 engines concurrently (good balance, moderate CPU)
/// - turbo: 3 engines concurrently (faster, higher CPU)
///
/// Note: these numbers are intentionally conservative. Each engine has
/// its own internal parallelism (e.g. clean engine runs up to 3-6 scan
/// categories concurrently, each via spawn_blocking). So "balanced" with
/// 2 engines × 3 internal tasks = up to 6 concurrent blocking threads,
/// which is a reasonable load for modern multi-core Macs.
fn max_concurrent_engines(resource_mode: &str) -> usize {
    match resource_mode {
        "eco" => 1,
        "turbo" => 3,
        _ => 2, // balanced
    }
}

/// Run a list of engine futures with **actually bounded** concurrency.
///
/// In eco mode (max_concurrent=1), engines run sequentially.
/// In balanced/turbo mode, at most `max_concurrent` engines run at once,
/// enforced by a tokio Semaphore. Each engine is spawned as a 'static task
/// that acquires a permit before running, so the semaphore truly limits
/// how many engines are active simultaneously.
///
/// This is critical for CPU strain: without real bounding, FuturesUnordered
/// polls ALL futures at once, which — combined with each engine's internal
/// parallelism (spawn_blocking for WalkDir) — can spawn dozens of blocking
/// threads and saturate all CPU cores.
async fn run_engines_concurrent(
    tasks: Vec<EngineFuture>,
    max_concurrent: usize,
) -> Vec<EngineResult> {
    if tasks.is_empty() {
        return Vec::new();
    }

    if max_concurrent <= 1 {
        // Eco mode: run sequentially — lowest CPU strain.
        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            results.push(task.await);
        }
        return results;
    }

    // Balanced/Turbo: use JoinSet + Semaphore for real bounded concurrency.
    // Each task is spawned as a 'static task that acquires a permit before
    // running. This ensures at most `max_concurrent` engines are active.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    let mut join_set = tokio::task::JoinSet::new();

    for task in tasks {
        let sem = semaphore.clone();
        join_set.spawn(async move {
            // Acquire permit — blocks if max_concurrent engines are already running.
            let _permit = sem.acquire().await.expect("semaphore closed");
            task.await
        });
    }

    let mut results = Vec::with_capacity(join_set.len());
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(engine_result) => results.push(engine_result),
            Err(join_err) => {
                tracing::error!("Engine task panicked: {}", join_err);
            }
        }
    }
    results
}

async fn run_all_engines(
    ctx: Arc<ScanContext>,
    args: &cli::args::AllArgs,
) -> Vec<std::result::Result<core::types::EngineStats, core::error::EngineError>> {
    use cli::args::EngineIdArg;

    let skip: Vec<EngineIdArg> = args.skip.clone();
    let should_run = |id: EngineIdArg| -> bool { !skip.contains(&id) };

    // Determine max concurrent engines from resource_mode.
    let max_concurrent = max_concurrent_engines(&ctx.config.resource_mode);

    // Build the list of engines to run. Each is a boxed future that returns
    // the engine's result. Engines are independent — they scan different
    // aspects of the system and stream findings via the shared mpsc channel.
    let mut tasks: Vec<EngineFuture> = Vec::new();

    if should_run(EngineIdArg::Clean) {
        let engine = engines::clean::CleanEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }
    if should_run(EngineIdArg::Conflict) {
        let engine = engines::conflict::ConflictEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }
    if should_run(EngineIdArg::Map) {
        let engine = engines::map::MapEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }
    if should_run(EngineIdArg::Envmap) {
        let engine = engines::envmap::EnvmapEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }
    if should_run(EngineIdArg::Depth) {
        let engine = engines::depth::DepthEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }

    run_engines_concurrent(tasks, max_concurrent).await
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

    let max_concurrent = max_concurrent_engines(&ctx.config.resource_mode);

    let mut tasks: Vec<EngineFuture> = Vec::new();

    if should_run(ScanEngineIdArg::Clean) {
        let engine = engines::clean::CleanEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }
    if should_run(ScanEngineIdArg::Conflict) {
        let engine = engines::conflict::ConflictEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }
    if should_run(ScanEngineIdArg::Map) {
        let engine = engines::map::MapEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }
    if args.envmap && should_run(ScanEngineIdArg::Envmap) {
        let engine = engines::envmap::EnvmapEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }
    if args.include_depth && should_run(ScanEngineIdArg::Depth) {
        let engine = engines::depth::DepthEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }
    if args.diagnostics && should_run(ScanEngineIdArg::Diag) {
        let engine = engines::diag::DiagEngine::default();
        let ctx = ctx.clone();
        tasks.push(Box::pin(async move { engine.run(ctx).await }));
    }

    run_engines_concurrent(tasks, max_concurrent).await
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
        eprintln!(
            "Installed: {} -> {}",
            target.display(),
            current_exe.display()
        );
        eprintln!("You can now run `xmac` from any directory.");
        eprintln!(
            "If '{}' is not on your PATH, add it to your shell profile:",
            install_dir.display()
        );
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
    use cleanup::{CleanupExecutor, CleanupPolicy};
    use cli::args::PurgeCategoryArg;

    let mut policy = CleanupPolicy::safe();
    if args.force_review {
        policy.allow_trash_overrides = true;
    }

    let xmac_config = config::ConfigManager::load();
    let xmac_config = xmac_config.config().clone();

    // 1. Run the clean scan to produce findings.
    let clean_args = cli::args::CleanArgs {
        min_age: args.min_age.clone(),
        min_size: args.min_size.clone(),
        ..engines::clean::CleanEngine::default_args()
    };

    let (tx, mut rx) = mpsc::channel::<Finding>(1000);
    let ctx = Arc::new(ScanContext::new(cli, tx).await?);
    let clean_engine = engines::clean::CleanEngine::new(clean_args).with_config(&xmac_config);
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
                PurgeCategoryArg::PackageManagerCache => {
                    crate::core::types::Category::PackageManagerCache
                }
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
    let xmac_config = config::ConfigManager::load();
    let xmac_config = xmac_config.config().clone();

    let max_concurrent = max_concurrent_engines(&ctx.config.resource_mode);

    let mut tasks: Vec<EngineFuture> = Vec::new();

    // 1. Clean scan (with dedup if requested)
    let clean_args = cli::args::CleanArgs {
        dedup: args.dedup,
        ..engines::clean::CleanEngine::default_args()
    };
    let clean_engine = engines::clean::CleanEngine::new(clean_args).with_config(&xmac_config);
    let ctx_clone = ctx.clone();
    tasks.push(Box::pin(async move { clean_engine.run(ctx_clone).await }));

    // 2. Maintenance tasks (safe ones only — no sudo)
    if !args.no_maintain {
        let maintain_engine =
            engines::maintain::MaintainEngine::default().with_config(&xmac_config);
        let ctx_clone = ctx.clone();
        tasks.push(Box::pin(
            async move { maintain_engine.run(ctx_clone).await },
        ));
    }

    // 3. Disk usage breakdown
    if !args.no_disk {
        let disk_args = cli::args::DiskArgs {
            top: 20,
            min_size: "100M".to_string(),
            paths: args.paths.clone(),
        };
        let disk_engine = engines::disk::DiskEngine::new(disk_args);
        let ctx_clone = ctx.clone();
        tasks.push(Box::pin(async move { disk_engine.run(ctx_clone).await }));
    }

    run_engines_concurrent(tasks, max_concurrent).await
}

/// The `ram-boost` command — memory optimizer with before/after comparison.
async fn run_ram_boost(cli: &Cli, args: cli::args::RamBoostArgs) -> Result<()> {
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::channel::<core::types::Finding>(1000);
    let ctx = Arc::new(ScanContext::new(cli, tx).await?);

    // Run the RAM boost pipeline
    let boost_handle = {
        let ctx = ctx.clone();
        tokio::spawn(
            async move { engines::maintain::MaintainEngine::run_ram_boost(args, ctx).await },
        )
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

/// The `optimize` command — memory telemetry, graph building, and pressure
/// prediction. In observe mode (default), collects a snapshot, builds the
/// memory graph, and emits findings with predictions. No actions are taken.
async fn run_optimize(cli: &Cli, args: cli::args::OptimizeArgs) -> Result<()> {
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::channel::<core::types::Finding>(1000);
    let ctx = Arc::new(ScanContext::new(cli, tx).await?);

    // Run the optimize engine
    let optimize_handle = {
        let ctx = ctx.clone();
        tokio::spawn(
            async move { engines::optimize::OptimizeEngine::run_optimize(args, ctx).await },
        )
    };

    drop(ctx);

    // Print findings as they arrive
    let is_json = cli.global.format == cli::args::OutputFormat::Json
        || cli.global.format == cli::args::OutputFormat::JsonPretty;

    let mut findings = Vec::new();
    while let Some(finding) = rx.recv().await {
        if is_json {
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

    optimize_handle.await??;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  Config command
// ═══════════════════════════════════════════════════════════════════════

fn run_config(cli: &Cli, args: &cli::args::ConfigArgs) -> Result<()> {
    use cli::args::{ConfigAction, OutputFormat};
    use config::{
        profiles::{OptimizationProfile, ProfilePreset},
        ConfigManager,
    };

    let json_out = cli.global.format == OutputFormat::Json;

    match &args.action {
        ConfigAction::Init => {
            let mgr = ConfigManager::load();
            let path = mgr.path().to_path_buf();
            mgr.ensure_config_file().map_err(|e| anyhow::anyhow!(e))?;
            if json_out {
                serde_json::to_writer_pretty(
                    std::io::stdout(),
                    &serde_json::json!({
                        "action": "init",
                        "path": path.display().to_string(),
                        "status": "created"
                    }),
                )?;
                println!();
            } else {
                eprintln!("Created config file at: {}", path.display());
                eprintln!("Edit it to customize X-MaC behavior.");
            }
            Ok(())
        }
        ConfigAction::Show => {
            let mgr = ConfigManager::load();
            if json_out {
                serde_json::to_writer_pretty(std::io::stdout(), mgr.config())?;
                println!();
            } else {
                let toml = toml::to_string_pretty(mgr.config()).map_err(|e| anyhow::anyhow!(e))?;
                println!("{}", toml);
            }
            Ok(())
        }
        ConfigAction::Path => {
            let path = ConfigManager::default_config_path();
            if json_out {
                serde_json::to_writer_pretty(
                    std::io::stdout(),
                    &serde_json::json!({
                        "config_path": path.display().to_string()
                    }),
                )?;
                println!();
            } else {
                println!("{}", path.display());
            }
            Ok(())
        }
        ConfigAction::Profiles => {
            if json_out {
                let profiles: Vec<_> = ProfilePreset::all()
                    .iter()
                    .map(|p| serde_json::json!({"name": p.name, "description": p.description}))
                    .collect();
                serde_json::to_writer_pretty(std::io::stdout(), &profiles)?;
                println!();
            } else {
                eprintln!("Available optimization profiles:\n");
                for preset in ProfilePreset::all() {
                    eprintln!("  {:15} {}", preset.name, preset.description);
                }
                eprintln!("\nSet with: xmac config set-profile <name>");
            }
            Ok(())
        }
        ConfigAction::SetProfile { name } => {
            let profile = match name.to_lowercase().as_str() {
                "balanced" => OptimizationProfile::Balanced,
                "gaming" => OptimizationProfile::Gaming,
                "development" | "dev" => OptimizationProfile::Development,
                "video" | "video_editing" | "videoediting" => OptimizationProfile::VideoEditing,
                "conservative" => OptimizationProfile::Conservative,
                "aggressive" => OptimizationProfile::Aggressive,
                "custom" => OptimizationProfile::Custom,
                _ => anyhow::bail!("Unknown profile '{}'. Available: balanced, gaming, development, video_editing, conservative, aggressive, custom", name),
            };
            let mut mgr = ConfigManager::load();
            mgr.set_profile(profile);
            mgr.save().map_err(|e| anyhow::anyhow!(e))?;
            if json_out {
                serde_json::to_writer_pretty(
                    std::io::stdout(),
                    &serde_json::json!({
                        "action": "set_profile",
                        "profile": name,
                        "label": profile.label(),
                        "description": profile.description(),
                        "status": "ok"
                    }),
                )?;
                println!();
            } else {
                eprintln!(
                    "Active profile set to: {} ({})",
                    profile.label(),
                    profile.description()
                );
            }
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut mgr = ConfigManager::load();
            set_config_value(mgr.config_mut(), key, value)?;
            mgr.save().map_err(|e| anyhow::anyhow!(e))?;
            if json_out {
                serde_json::to_writer_pretty(
                    std::io::stdout(),
                    &serde_json::json!({
                        "action": "set", "key": key, "value": value, "status": "ok"
                    }),
                )?;
                println!();
            } else {
                eprintln!("Set {} = {}", key, value);
            }
            Ok(())
        }
        ConfigAction::Get { key } => {
            let mgr = ConfigManager::load();
            let val = get_config_value(mgr.config(), key);
            if json_out {
                serde_json::to_writer_pretty(
                    std::io::stdout(),
                    &serde_json::json!({
                        "key": key, "value": val
                    }),
                )?;
                println!();
            } else {
                println!("{}", val);
            }
            Ok(())
        }
    }
}

fn set_config_value(config: &mut config::Config, key: &str, value: &str) -> Result<()> {
    use config::profiles::OptimizationProfile;
    match key {
        "profile" => {
            config.profile = match value.to_lowercase().as_str() {
                "balanced" => OptimizationProfile::Balanced,
                "gaming" => OptimizationProfile::Gaming,
                "development" | "dev" => OptimizationProfile::Development,
                "video_editing" | "videoediting" => OptimizationProfile::VideoEditing,
                "conservative" => OptimizationProfile::Conservative,
                "aggressive" => OptimizationProfile::Aggressive,
                "custom" => OptimizationProfile::Custom,
                _ => anyhow::bail!("Unknown profile: {}", value),
            };
        }
        "clean.min_age_days" => {
            config.clean.min_age_days = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "clean.min_size_mb" => {
            config.clean.min_size_mb = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "clean.dedup" => config.clean.dedup = parse_bool(value)?,
        "clean.xcode" => config.clean.xcode = parse_bool(value)?,
        "clean.build_artifacts" => config.clean.build_artifacts = parse_bool(value)?,
        "clean.browser" => config.clean.browser = parse_bool(value)?,
        "clean.large_files" => config.clean.large_files = parse_bool(value)?,
        "clean.min_large_size_mb" => {
            config.clean.min_large_size_mb = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "duplicate.min_size" => {
            config.duplicate.min_size = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "duplicate.enabled" => config.duplicate.enabled = parse_bool(value)?,
        "duplicate.similar_images" => config.duplicate.similar_images = parse_bool(value)?,
        "maintain.dns" => config.maintain.dns = parse_bool(value)?,
        "maintain.spotlight" => config.maintain.spotlight = parse_bool(value)?,
        "maintain.launchservices" => config.maintain.launchservices = parse_bool(value)?,
        "maintain.purge_ram" => config.maintain.purge_ram = parse_bool(value)?,
        "maintain.quicklook" => config.maintain.quicklook = parse_bool(value)?,
        "optimize.max_processes" => {
            config.optimize.max_processes = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "optimize.exclude_system" => config.optimize.exclude_system = parse_bool(value)?,
        "optimize.buffer_size" => {
            config.optimize.buffer_size = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "optimize.top_n" => {
            config.optimize.top_n = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "optimize.proactive_predictions" => {
            config.optimize.proactive_predictions = parse_bool(value)?
        }
        "optimize.pressure_threshold" => {
            config.optimize.pressure_threshold = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "optimize.ai_advisor" => config.optimize.ai_advisor = parse_bool(value)?,
        "daemon.enabled" => config.daemon.enabled = parse_bool(value)?,
        "daemon.interval_secs" => {
            config.daemon.interval_secs = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "daemon.auto_clean_threshold_mb" => {
            config.daemon.auto_clean_threshold_mb = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "daemon.auto_purge_memory" => config.daemon.auto_purge_memory = parse_bool(value)?,
        "daemon.collect_telemetry" => config.daemon.collect_telemetry = parse_bool(value)?,
        "daemon.telemetry_interval_secs" => {
            config.daemon.telemetry_interval_secs = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "notifications.enabled" => config.notifications.enabled = parse_bool(value)?,
        "notifications.memory_pressure" => {
            config.notifications.memory_pressure = parse_bool(value)?
        }
        "notifications.reclaimable_space" => {
            config.notifications.reclaimable_space = parse_bool(value)?
        }
        "notifications.space_threshold_mb" => {
            config.notifications.space_threshold_mb = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "notifications.proactive_warnings" => {
            config.notifications.proactive_warnings = parse_bool(value)?
        }
        "history.max_snapshots" => {
            config.history.max_snapshots = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "history.max_transactions" => {
            config.history.max_transactions = value
                .parse()
                .map_err(|e| anyhow::anyhow!("parse error: {}", e))?
        }
        "logging.level" => config.logging.level = value.to_string(),
        "logging.file_logging" => config.logging.file_logging = parse_bool(value)?,
        _ => anyhow::bail!(
            "Unknown config key: '{}'. Use 'xmac config show' to see available keys.",
            key
        ),
    }
    Ok(())
}

fn get_config_value(config: &config::Config, key: &str) -> String {
    match key {
        "profile" => format!("{:?}", config.profile),
        "clean.min_age_days" => config.clean.min_age_days.to_string(),
        "clean.min_size_mb" => config.clean.min_size_mb.to_string(),
        "clean.dedup" => config.clean.dedup.to_string(),
        "clean.xcode" => config.clean.xcode.to_string(),
        "clean.build_artifacts" => config.clean.build_artifacts.to_string(),
        "clean.browser" => config.clean.browser.to_string(),
        "clean.large_files" => config.clean.large_files.to_string(),
        "clean.min_large_size_mb" => config.clean.min_large_size_mb.to_string(),
        "duplicate.min_size" => config.duplicate.min_size.to_string(),
        "duplicate.enabled" => config.duplicate.enabled.to_string(),
        "duplicate.similar_images" => config.duplicate.similar_images.to_string(),
        "optimize.max_processes" => config.optimize.max_processes.to_string(),
        "optimize.pressure_threshold" => config.optimize.pressure_threshold.to_string(),
        "optimize.ai_advisor" => config.optimize.ai_advisor.to_string(),
        "daemon.enabled" => config.daemon.enabled.to_string(),
        "daemon.interval_secs" => config.daemon.interval_secs.to_string(),
        "daemon.auto_purge_memory" => config.daemon.auto_purge_memory.to_string(),
        "notifications.enabled" => config.notifications.enabled.to_string(),
        "logging.level" => config.logging.level.clone(),
        _ => format!("(unknown key: {})", key),
    }
}

fn parse_bool(s: &str) -> Result<bool> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => anyhow::bail!("Expected boolean (true/false), got: {}", s),
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Daemon command
// ═══════════════════════════════════════════════════════════════════════

async fn run_daemon(_cli: &Cli, args: cli::args::DaemonArgs) -> Result<()> {
    use config::ConfigManager;
    use intelligence::daemon::Daemon;

    let mgr = ConfigManager::load();

    if args.status {
        match Daemon::is_running(&mgr.config().daemon.pid_file) {
            Some(pid) => {
                eprintln!("xmac daemon is running (pid {})", pid);
            }
            None => {
                eprintln!("xmac daemon is not running");
            }
        }
        return Ok(());
    }

    if args.stop {
        Daemon::stop(&mgr.config().daemon.pid_file)?;
        eprintln!("xmac daemon stopped");
        return Ok(());
    }

    let interval = args.interval.unwrap_or(mgr.config().daemon.interval_secs);
    let daemon = Daemon::new(mgr, interval, args.daemon_verbose);
    daemon.run(args.once).await
}

// ═══════════════════════════════════════════════════════════════════════
//  Zen Mode command
// ═══════════════════════════════════════════════════════════════════════

async fn run_zen(cli: &Cli, args: &cli::args::ZenArgs) -> Result<()> {
    let is_json =
        cli.global.format == OutputFormat::Json || cli.global.format == OutputFormat::JsonPretty;

    let result = intelligence::zen::run_zen(cli, args).await?;

    if is_json {
        let json = match cli.global.format {
            OutputFormat::JsonPretty => serde_json::to_string_pretty(&result)?,
            _ => serde_json::to_string(&result)?,
        };
        println!("{}", json);
    } else {
        print!(
            "{}",
            intelligence::zen::format_zen_result_text(&result, args.dry_run || !args.execute)
        );
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  Advisor command
// ═══════════════════════════════════════════════════════════════════════

async fn run_advisor(_cli: &Cli, args: &cli::args::AdvisorArgs) -> Result<()> {
    use config::ConfigManager;
    use intelligence::advisor::{format_recommendations_text, Advisor, Severity};

    let mgr = ConfigManager::load();
    let config = mgr.config();

    // Collect system snapshot
    let snapshot = intelligence::SystemSnapshot::collect();

    // Create advisor with current profile and adaptive state
    let advisor = Advisor::new(config.profile, config.adaptive.clone());
    let mut recs = advisor.analyze(&snapshot);

    // Filter by min severity
    if let Some(min_sev) = Severity::parse_severity(&args.min_severity) {
        recs.retain(|r| r.severity >= min_sev);
    }

    // Limit to top N
    recs.truncate(args.top);

    if args.advisor_format == "json" {
        let json = serde_json::to_string_pretty(&recs)?;
        if args.health_score {
            let output = serde_json::json!({
                "health_score": snapshot.health_score,
                "status": snapshot.status,
                "recommendations": recs,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("{}", json);
        }
    } else {
        let health = if args.health_score {
            Some(snapshot.health_score)
        } else {
            None
        };
        let text = format_recommendations_text(&recs, health);
        print!("{}", text);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  History command
// ═══════════════════════════════════════════════════════════════════════

fn run_history(cli: &Cli, args: &cli::args::HistoryArgs) -> Result<()> {
    use cleanup::history::{load_history, save_history};
    use cli::args::OutputFormat;
    use config::ConfigManager;

    let json_out = cli.global.format == OutputFormat::Json;

    let mgr = ConfigManager::load();
    let history_path = &mgr.config().history.path;

    if args.clear {
        let empty = cleanup::history::CleanupHistory::new();
        save_history(&empty, history_path).map_err(|e| anyhow::anyhow!(e))?;
        if json_out {
            serde_json::to_writer_pretty(
                std::io::stdout(),
                &serde_json::json!({
                    "action": "clear", "status": "ok"
                }),
            )?;
            println!();
        } else {
            eprintln!("History cleared.");
        }
        return Ok(());
    }

    let history = load_history(history_path);

    if let Some(export_path) = &args.export {
        let json = serde_json::to_string_pretty(&history)?;
        std::fs::write(export_path, json)?;
        if json_out {
            serde_json::to_writer_pretty(
                std::io::stdout(),
                &serde_json::json!({
                    "action": "export", "path": export_path.display().to_string(), "status": "ok"
                }),
            )?;
            println!();
        } else {
            eprintln!("History exported to: {}", export_path.display());
        }
        return Ok(());
    }

    if json_out {
        // Output full history as JSON
        serde_json::to_writer_pretty(std::io::stdout(), &history)?;
        println!();
        return Ok(());
    }

    if args.summary {
        let total_reclaimed: u64 = history
            .transactions
            .iter()
            .map(|t| t.successful_bytes())
            .sum();
        let total_snapshots = history.snapshots.len();
        let total_transactions = history.transactions.len();

        eprintln!("X-MaC History Summary");
        eprintln!("════════════════════════════════════════");
        eprintln!("  Total scans:        {}", total_snapshots);
        eprintln!("  Total cleanups:     {}", total_transactions);
        eprintln!(
            "  Total reclaimed:    {}",
            crate::util::disk::format_bytes(total_reclaimed)
        );
        eprintln!();

        if !history.snapshots.is_empty() {
            let first = history.snapshots.first().unwrap();
            let last = history.snapshots.last().unwrap();
            eprintln!(
                "  First scan:         {} (reclaimable: {})",
                chrono::DateTime::from_timestamp(first.timestamp as i64, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| first.timestamp.to_string()),
                crate::util::disk::format_bytes(first.reclaimable_bytes)
            );
            eprintln!(
                "  Last scan:          {} (reclaimable: {})",
                chrono::DateTime::from_timestamp(last.timestamp as i64, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| last.timestamp.to_string()),
                crate::util::disk::format_bytes(last.reclaimable_bytes)
            );
        }
        return Ok(());
    }

    // Show recent entries
    eprintln!("X-MaC History (last {} entries)\n", args.last);

    let transactions: Vec<_> = history.transactions.iter().rev().take(args.last).collect();
    if transactions.is_empty() {
        eprintln!("  No cleanup history yet. Run 'xmac purge' to start building history.");
    } else {
        for t in transactions {
            let dt = chrono::DateTime::from_timestamp(t.started_at as i64, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| t.started_at.to_string());
            eprintln!(
                "  {} | reclaimed {} | {} actions",
                dt,
                crate::util::disk::format_bytes(t.successful_bytes()),
                t.successful_count()
            );
        }
    }

    Ok(())
}

/// Generate shell completion scripts.
fn run_completions(args: cli::args::CompletionsArgs) -> Result<()> {
    use clap::CommandFactory;
    use cli::args::ShellArg;
    let mut cmd = cli::args::Cli::command();
    let shell = match args.shell {
        ShellArg::Bash => clap_complete::Shell::Bash,
        ShellArg::Zsh => clap_complete::Shell::Zsh,
        ShellArg::Fish => clap_complete::Shell::Fish,
        ShellArg::Elvish => clap_complete::Shell::Elvish,
        ShellArg::PowerShell => clap_complete::Shell::PowerShell,
    };
    clap_complete::generate(shell, &mut cmd, "xmac", &mut std::io::stdout());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  Digital Twin
// ═══════════════════════════════════════════════════════════════════════

async fn run_twin(cli: &Cli, args: cli::args::TwinArgs) -> Result<()> {
    use cli::args::{OutputFormat, TwinAction};

    let json_out = cli.global.format == OutputFormat::Json;

    fn print_json<T: serde::Serialize>(value: &T, json_out: bool) -> Result<()> {
        if json_out {
            serde_json::to_writer_pretty(std::io::stdout(), value)
                .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))?;
        } else {
            let json_str = serde_json::to_string_pretty(value)
                .unwrap_or_else(|_| "<serialization error>".to_string());
            println!("{}", json_str);
        }
        Ok(())
    }

    match args.action {
        TwinAction::Collect => {
            eprintln!("Collecting Digital Twin snapshot...");
            let twin = twin::DigitalTwin::collect();
            print_json(&twin, json_out)?;
        }
        TwinAction::Ask => {
            let question = args
                .question
                .as_deref()
                .unwrap_or("How is my system doing?");
            eprintln!("Collecting twin and reasoning...");
            let twin = twin::DigitalTwin::collect();
            let engine = twin.reason();
            let result = engine.ask(question);
            print_json(&result, json_out)?;
        }
        TwinAction::Predict => {
            eprintln!("Collecting twin and predicting problems...");
            let twin = twin::DigitalTwin::collect();
            let engine = twin.reason();
            let predictions = engine.predict_problems();
            print_json(&predictions, json_out)?;
        }
        TwinAction::Simulate => {
            let action = args.simulate.as_deref().unwrap_or("clear cache");
            eprintln!("Simulating: {}...", action);
            let result = twin::reasoning::ReasoningEngine::sandbox_simulation(action);
            print_json(&result, json_out)?;
        }
        TwinAction::Recommend => {
            eprintln!("Collecting twin and generating recommendations...");
            let twin = twin::DigitalTwin::collect();
            let engine = twin.reason();
            let mut recommendations = serde_json::json!({
                "cleanup_impact": engine.simulate_cleanup(),
                "workflow_changes": engine.recommend_workflow_changes(),
                "hardware_upgrades": engine.recommend_hardware_upgrades(),
                "software_changes": engine.recommend_software_changes(),
                "preventive_actions": engine.recommend_preventive_actions(),
            });
            if let Some(obj) = recommendations.as_object_mut() {
                obj.insert(
                    "health_score".to_string(),
                    serde_json::json!(twin.health_score),
                );
                obj.insert(
                    "trust_score".to_string(),
                    serde_json::json!(twin.trust_score),
                );
            }
            print_json(&recommendations, json_out)?;
        }
        TwinAction::Query => {
            let dimension = args.query.as_deref().unwrap_or("health");
            eprintln!("Querying dimension: {}...", dimension);
            let twin = twin::DigitalTwin::collect();
            let engine = twin.reason();
            let result = engine.query(dimension);
            print_json(&result, json_out)?;
        }
        TwinAction::Benchmark => {
            eprintln!("Generating anonymized benchmark...");
            let twin = twin::DigitalTwin::collect();
            let engine = twin.reason();
            let benchmark = engine.generate_anonymized_benchmark();
            print_json(&benchmark, json_out)?;
        }
        TwinAction::Monitor => {
            eprintln!("Generating monitoring plan...");
            let twin = twin::DigitalTwin::collect();
            let engine = twin.reason();
            let plan = engine.continuous_monitoring_plan();
            print_json(&plan, json_out)?;
        }
        TwinAction::InitDb => {
            let db_path = twin::database::default_db_path()?;
            eprintln!(
                "Initializing Digital Twin database at {}",
                db_path.display()
            );
            let db = twin::database::TwinDb::open(&db_path)?;
            // Verify tables exist.
            let conn = db.conn().await;
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |r| r.get(0),
            )?;
            eprintln!("Database initialized. {} tables ready.", count);
            if json_out {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "initialized",
                        "path": db_path.display().to_string(),
                        "tables": count
                    })
                );
            }
        }
        TwinAction::WhatChanged => {
            let db_path = twin::database::default_db_path()?;
            let db = twin::database::TwinDb::open(&db_path)?;
            let store = twin::database::EventStore::new(db.handle());

            let end_ms = args
                .until
                .as_deref()
                .map(parse_timestamp)
                .transpose()?
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
            let start_ms = args
                .since
                .as_deref()
                .map(parse_timestamp)
                .transpose()?
                .unwrap_or_else(||
                // Default: 24 hours ago.
                chrono::Utc::now().timestamp_millis() - 86_400_000);

            eprintln!(
                "Querying changes from {} to {}...",
                chrono::DateTime::<chrono::Utc>::from_timestamp_millis(start_ms)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| start_ms.to_string()),
                chrono::DateTime::<chrono::Utc>::from_timestamp_millis(end_ms)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| end_ms.to_string()),
            );

            let report = twin::what_changed::what_changed_between(&store, start_ms, end_ms).await?;
            if json_out {
                print_json(&report, true)?;
            } else {
                print!("{}", twin::what_changed::format_report(&report));
            }
        }
        TwinAction::Compact => {
            let db_path = twin::database::default_db_path()?;
            let db = twin::database::TwinDb::open(&db_path)?;
            let store = twin::database::EventStore::new(db.handle());
            eprintln!("Running compaction (raw events older than 7 days → hourly aggregates)...");
            let (hourly_pruned, raw_deleted) = store.compact_events(7).await?;
            let event_count = store.event_count().await?;
            let agg_count = store.aggregate_count().await?;
            eprintln!(
                "Compaction complete: {} raw events deleted, {} hourly aggregates pruned.",
                raw_deleted, hourly_pruned
            );
            eprintln!(
                "Current: {} raw events, {} aggregates.",
                event_count, agg_count
            );
            if json_out {
                println!(
                    "{}",
                    serde_json::json!({
                        "raw_events_deleted": raw_deleted,
                        "hourly_aggregates_pruned": hourly_pruned,
                        "remaining_events": event_count,
                        "remaining_aggregates": agg_count,
                    })
                );
            }
        }
        TwinAction::Observe => {
            let db_path = twin::database::default_db_path()?;
            let db = twin::database::TwinDb::open(&db_path)?;
            let duration = parse_duration(&args.duration)?;
            eprintln!(
                "Starting observers for {} (writing to {})",
                humanize_duration(duration),
                db_path.display()
            );
            eprintln!("Watching processes (5s poll) and filesystem (FSEvents).");
            eprintln!("Press Ctrl-C to stop early.\n");

            let runner = twin::observers::ObserverRunner::new(db.handle());
            let stats = runner.run_for(duration).await?;

            eprintln!("\nObserver stats:");
            eprintln!("  Total events:    {}", stats.total_events);
            eprintln!("  Process events:  {}", stats.process_events);
            eprintln!("  FS events:       {}", stats.fs_events);
            eprintln!("  Poll cycles:     {}", stats.poll_cycles);

            if json_out {
                println!(
                    "{}",
                    serde_json::json!({
                        "total_events": stats.total_events,
                        "process_events": stats.process_events,
                        "fs_events": stats.fs_events,
                        "poll_cycles": stats.poll_cycles,
                    })
                );
            }
        }
    }

    Ok(())
}

/// Parse a timestamp string. Supports:
///   - ISO 8601: "2026-07-14T13:00:00Z"
///   - Relative: "7d" (7 days ago), "24h" (24 hours ago), "30m" (30 minutes ago)
fn parse_timestamp(s: &str) -> Result<i64> {
    // Try relative format first: NNd, NNh, NNm.
    if let Some(rest) = s.strip_suffix('d') {
        if let Ok(n) = rest.parse::<i64>() {
            return Ok(chrono::Utc::now().timestamp_millis() - n * 86_400_000);
        }
    }
    if let Some(rest) = s.strip_suffix('h') {
        if let Ok(n) = rest.parse::<i64>() {
            return Ok(chrono::Utc::now().timestamp_millis() - n * 3_600_000);
        }
    }
    if let Some(rest) = s.strip_suffix('m') {
        if let Ok(n) = rest.parse::<i64>() {
            return Ok(chrono::Utc::now().timestamp_millis() - n * 60_000);
        }
    }
    // Try ISO 8601.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&chrono::Utc).timestamp_millis());
    }
    // Try YYYY-MM-DD HH:MM.
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        return Ok(dt.and_utc().timestamp_millis());
    }
    // Try YYYY-MM-DD.
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis());
    }
    // Try plain integer (epoch millis).
    if let Ok(ms) = s.parse::<i64>() {
        return Ok(ms);
    }
    anyhow::bail!(
        "invalid timestamp: {} (use ISO 8601, relative like '7d', or epoch millis)",
        s
    )
}

/// Parse a duration string. Supports: "60s", "5m", "1h", "30s".
fn parse_duration(s: &str) -> Result<std::time::Duration> {
    if let Some(rest) = s.strip_suffix('s') {
        if let Ok(n) = rest.parse::<u64>() {
            return Ok(std::time::Duration::from_secs(n));
        }
    }
    if let Some(rest) = s.strip_suffix('m') {
        if let Ok(n) = rest.parse::<u64>() {
            return Ok(std::time::Duration::from_secs(n * 60));
        }
    }
    if let Some(rest) = s.strip_suffix('h') {
        if let Ok(n) = rest.parse::<u64>() {
            return Ok(std::time::Duration::from_secs(n * 3600));
        }
    }
    // Try plain seconds.
    if let Ok(n) = s.parse::<u64>() {
        return Ok(std::time::Duration::from_secs(n));
    }
    anyhow::bail!(
        "invalid duration: {} (use format like '60s', '5m', '1h')",
        s
    )
}

/// Human-readable duration for display.
fn humanize_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs >= 3600 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Safety command
// ═══════════════════════════════════════════════════════════════════════

fn run_safety(cli: &Cli, args: cli::args::SafetyArgs) -> Result<()> {
    use cli::args::SafetyAction;
    use safety::rule_engine::SafetyEngine;

    let engine = SafetyEngine::load_default().context("Failed to load safety rules")?;

    match args.action {
        SafetyAction::List => {
            let counts = engine.rule_counts();
            if cli.global.format == cli::args::OutputFormat::Json
                || cli.global.format == cli::args::OutputFormat::JsonPretty
            {
                let output = serde_json::json!({
                    "total_rules": engine.rules().len(),
                    "counts_by_rating": counts,
                    "rules": engine.rules().iter().map(|r| serde_json::json!({
                        "name": r.name,
                        "description": r.description,
                        "rating": r.rating.label(),
                        "paths": r.paths,
                        "confidence": r.confidence,
                        "category": r.category,
                        "upstream_commit": r.upstream_commit,
                    })).collect::<Vec<_>>()
                });
                if cli.global.format == cli::args::OutputFormat::JsonPretty {
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    println!("{}", serde_json::to_string(&output)?);
                }
            } else {
                println!("Safety Rules ({} total)", engine.rules().len());
                println!(
                    "  Safe: {} | Review: {} | Protected: {}",
                    counts.get("safe").copied().unwrap_or(0),
                    counts.get("review").copied().unwrap_or(0),
                    counts.get("protected").copied().unwrap_or(0),
                );
                println!();
                for rule in engine.rules() {
                    println!(
                        "  [{}] {} (confidence: {}/100)",
                        rule.rating.label(),
                        rule.name,
                        rule.confidence,
                    );
                    println!("    {}", rule.description);
                    for path in &rule.paths {
                        println!("    → {}", path);
                    }
                    println!();
                }
            }
        }
        SafetyAction::Classify => {
            let path = args
                .path
                .context("--path is required for --action classify")?;
            match engine.classify(&path) {
                Some(classification) => {
                    if cli.global.format == cli::args::OutputFormat::Json
                        || cli.global.format == cli::args::OutputFormat::JsonPretty
                    {
                        let output = serde_json::json!({
                            "path": classification.path,
                            "rating": classification.rating.label(),
                            "rule": classification.rule_name,
                            "description": classification.rule_description,
                            "confidence": classification.confidence,
                            "category": classification.category,
                            "preselected": classification.preselected,
                            "explanation": classification.explanation(),
                        });
                        if cli.global.format == cli::args::OutputFormat::JsonPretty {
                            println!("{}", serde_json::to_string_pretty(&output)?);
                        } else {
                            println!("{}", serde_json::to_string(&output)?);
                        }
                    } else {
                        println!("Path: {}", classification.path);
                        println!("Rating: {}", classification.rating.label());
                        println!("Rule: {}", classification.rule_name);
                        println!("Description: {}", classification.rule_description);
                        println!("Confidence: {}/100", classification.confidence);
                        println!("Preselected: {}", classification.preselected);
                    }
                }
                None => {
                    if cli.global.format == cli::args::OutputFormat::Json
                        || cli.global.format == cli::args::OutputFormat::JsonPretty
                    {
                        println!(
                            "{}",
                            serde_json::json!({
                                "path": path,
                                "rating": "unclassified",
                                "rule": null,
                                "description": "No matching safety rule found",
                                "confidence": 0,
                                "preselected": false
                            })
                        );
                    } else {
                        println!("Path: {}", path);
                        println!("Rating: unclassified (no matching rule)");
                    }
                }
            }
        }
        SafetyAction::Preview => {
            let counts = engine.rule_counts();
            let profile_rules: Vec<_> = engine
                .rules()
                .iter()
                .filter(|r| args.profile == "all" || r.category.as_deref() == Some(&args.profile))
                .collect();

            if cli.global.format == cli::args::OutputFormat::Json
                || cli.global.format == cli::args::OutputFormat::JsonPretty
            {
                let output = serde_json::json!({
                    "profile": args.profile,
                    "total_rules": engine.rules().len(),
                    "matching_rules": profile_rules.len(),
                    "safe_rules": counts.get("safe").copied().unwrap_or(0),
                    "review_rules": counts.get("review").copied().unwrap_or(0),
                    "protected_rules": counts.get("protected").copied().unwrap_or(0),
                    "rules": profile_rules.iter().map(|r| serde_json::json!({
                        "name": r.name,
                        "rating": r.rating.label(),
                        "description": r.description,
                        "confidence": r.confidence,
                    })).collect::<Vec<_>>()
                });
                if cli.global.format == cli::args::OutputFormat::JsonPretty {
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    println!("{}", serde_json::to_string(&output)?);
                }
            } else {
                println!("Cleanup Preview (profile: {})", args.profile);
                println!(
                    "  {} rules match (of {} total)",
                    profile_rules.len(),
                    engine.rules().len()
                );
                println!();
                for rule in &profile_rules {
                    println!(
                        "  [{}] {} — {}",
                        rule.rating.label(),
                        rule.name,
                        rule.description
                    );
                }
            }
        }
    }

    Ok(())
}
