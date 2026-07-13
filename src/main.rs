use anyhow::Result;
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

    // Load config once — used to apply profile-based overrides to engines
    let xmac_config = config::ConfigManager::load();
    let xmac_config = xmac_config.config().clone();

    let engine_results = match &cli.command {
        cli::args::Commands::Quick(args) => {
            run_quick(ctx.clone(), args).await
        }
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
            let engine = engines::maintain::MaintainEngine::new(args.clone()).with_config(&xmac_config);
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

    let xmac_config = config::ConfigManager::load();
    let xmac_config = xmac_config.config().clone();

    // 1. Clean scan (with dedup if requested)
    let clean_args = cli::args::CleanArgs {
        dedup: args.dedup,
        ..engines::clean::CleanEngine::default_args()
    };
    let clean_engine = engines::clean::CleanEngine::new(clean_args).with_config(&xmac_config);
    results.push(clean_engine.run(ctx.clone()).await);

    // 2. Maintenance tasks (safe ones only — no sudo)
    if !args.no_maintain {
        let maintain_engine = engines::maintain::MaintainEngine::default().with_config(&xmac_config);
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

/// The `optimize` command — memory telemetry, graph building, and pressure
/// prediction. In observe mode (default), collects a snapshot, builds the
/// memory graph, and emits findings with predictions. No actions are taken.
async fn run_optimize(
    cli: &Cli,
    args: cli::args::OptimizeArgs,
) -> Result<()> {
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::channel::<core::types::Finding>(1000);
    let ctx = Arc::new(ScanContext::new(cli, tx).await?);

    // Run the optimize engine
    let optimize_handle = {
        let ctx = ctx.clone();
        tokio::spawn(async move {
            engines::optimize::OptimizeEngine::run_optimize(args, ctx).await
        })
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

fn run_config(_cli: &Cli, args: &cli::args::ConfigArgs) -> Result<()> {
    use cli::args::ConfigAction;
    use config::{ConfigManager, profiles::{OptimizationProfile, ProfilePreset}};

    match &args.action {
        ConfigAction::Init => {
            let mgr = ConfigManager::load();
            let path = mgr.path().to_path_buf();
            mgr.ensure_config_file().map_err(|e| anyhow::anyhow!(e))?;
            eprintln!("Created config file at: {}", path.display());
            eprintln!("Edit it to customize X-MaC behavior.");
            Ok(())
        }
        ConfigAction::Show => {
            let mgr = ConfigManager::load();
            let toml = toml::to_string_pretty(mgr.config()).map_err(|e| anyhow::anyhow!(e))?;
            println!("{}", toml);
            Ok(())
        }
        ConfigAction::Path => {
            println!("{}", ConfigManager::default_config_path().display());
            Ok(())
        }
        ConfigAction::Profiles => {
            eprintln!("Available optimization profiles:\n");
            for preset in ProfilePreset::all() {
                eprintln!("  {:15} {}", preset.name, preset.description);
            }
            eprintln!("\nSet with: xmac config set-profile <name>");
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
            eprintln!("Active profile set to: {} ({})", profile.label(), profile.description());
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut mgr = ConfigManager::load();
            set_config_value(mgr.config_mut(), key, value)?;
            mgr.save().map_err(|e| anyhow::anyhow!(e))?;
            eprintln!("Set {} = {}", key, value);
            Ok(())
        }
        ConfigAction::Get { key } => {
            let mgr = ConfigManager::load();
            let val = get_config_value(mgr.config(), key);
            println!("{}", val);
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
        "clean.min_age_days" => config.clean.min_age_days = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "clean.min_size_mb" => config.clean.min_size_mb = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "clean.dedup" => config.clean.dedup = parse_bool(value)?,
        "clean.xcode" => config.clean.xcode = parse_bool(value)?,
        "clean.build_artifacts" => config.clean.build_artifacts = parse_bool(value)?,
        "clean.browser" => config.clean.browser = parse_bool(value)?,
        "clean.large_files" => config.clean.large_files = parse_bool(value)?,
        "clean.min_large_size_mb" => config.clean.min_large_size_mb = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "maintain.dns" => config.maintain.dns = parse_bool(value)?,
        "maintain.spotlight" => config.maintain.spotlight = parse_bool(value)?,
        "maintain.launchservices" => config.maintain.launchservices = parse_bool(value)?,
        "maintain.purge_ram" => config.maintain.purge_ram = parse_bool(value)?,
        "maintain.quicklook" => config.maintain.quicklook = parse_bool(value)?,
        "optimize.max_processes" => config.optimize.max_processes = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "optimize.exclude_system" => config.optimize.exclude_system = parse_bool(value)?,
        "optimize.buffer_size" => config.optimize.buffer_size = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "optimize.top_n" => config.optimize.top_n = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "optimize.proactive_predictions" => config.optimize.proactive_predictions = parse_bool(value)?,
        "optimize.pressure_threshold" => config.optimize.pressure_threshold = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "optimize.ai_advisor" => config.optimize.ai_advisor = parse_bool(value)?,
        "daemon.enabled" => config.daemon.enabled = parse_bool(value)?,
        "daemon.interval_secs" => config.daemon.interval_secs = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "daemon.auto_clean_threshold_mb" => config.daemon.auto_clean_threshold_mb = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "daemon.auto_purge_memory" => config.daemon.auto_purge_memory = parse_bool(value)?,
        "daemon.collect_telemetry" => config.daemon.collect_telemetry = parse_bool(value)?,
        "daemon.telemetry_interval_secs" => config.daemon.telemetry_interval_secs = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "notifications.enabled" => config.notifications.enabled = parse_bool(value)?,
        "notifications.memory_pressure" => config.notifications.memory_pressure = parse_bool(value)?,
        "notifications.reclaimable_space" => config.notifications.reclaimable_space = parse_bool(value)?,
        "notifications.space_threshold_mb" => config.notifications.space_threshold_mb = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "notifications.proactive_warnings" => config.notifications.proactive_warnings = parse_bool(value)?,
        "history.max_snapshots" => config.history.max_snapshots = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "history.max_transactions" => config.history.max_transactions = value.parse().map_err(|e| anyhow::anyhow!("parse error: {}", e))?,
        "logging.level" => config.logging.level = value.to_string(),
        "logging.file_logging" => config.logging.file_logging = parse_bool(value)?,
        _ => anyhow::bail!("Unknown config key: '{}'. Use 'xmac config show' to see available keys.", key),
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
    let is_json = cli.global.format == OutputFormat::Json
        || cli.global.format == OutputFormat::JsonPretty;

    let result = intelligence::zen::run_zen(cli, args).await?;

    if is_json {
        let json = match cli.global.format {
            OutputFormat::JsonPretty => serde_json::to_string_pretty(&result)?,
            _ => serde_json::to_string(&result)?,
        };
        println!("{}", json);
    } else {
        print!("{}", intelligence::zen::format_zen_result_text(&result, args.dry_run || !args.execute));
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  Advisor command
// ═══════════════════════════════════════════════════════════════════════

async fn run_advisor(_cli: &Cli, args: &cli::args::AdvisorArgs) -> Result<()> {
    use config::ConfigManager;
    use intelligence::advisor::{Advisor, Severity, format_recommendations_text};

    let mgr = ConfigManager::load();
    let config = mgr.config();

    // Collect system snapshot
    let snapshot = intelligence::SystemSnapshot::collect();

    // Create advisor with current profile and adaptive state
    let advisor = Advisor::new(config.profile, config.adaptive.clone());
    let mut recs = advisor.analyze(&snapshot);

    // Filter by min severity
    if let Some(min_sev) = Severity::from_str(&args.min_severity) {
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
        let health = if args.health_score { Some(snapshot.health_score) } else { None };
        let text = format_recommendations_text(&recs, health);
    print!("{}", text);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  History command
// ═══════════════════════════════════════════════════════════════════════

fn run_history(_cli: &Cli, args: &cli::args::HistoryArgs) -> Result<()> {
    use config::ConfigManager;
    use cleanup::history::{load_history, save_history};

    let mgr = ConfigManager::load();
    let history_path = &mgr.config().history.path;

    if args.clear {
        let empty = cleanup::history::CleanupHistory::new();
        save_history(&empty, history_path).map_err(|e| anyhow::anyhow!(e))?;
        eprintln!("History cleared.");
        return Ok(());
    }

    let history = load_history(history_path);

    if let Some(export_path) = &args.export {
        let json = serde_json::to_string_pretty(&history)?;
        std::fs::write(export_path, json)?;
        eprintln!("History exported to: {}", export_path.display());
        return Ok(());
    }

    if args.summary {
        let total_reclaimed: u64 = history.transactions.iter()
            .map(|t| t.successful_bytes())
            .sum();
        let total_snapshots = history.snapshots.len();
        let total_transactions = history.transactions.len();

        eprintln!("X-MaC History Summary");
        eprintln!("════════════════════════════════════════");
        eprintln!("  Total scans:        {}", total_snapshots);
        eprintln!("  Total cleanups:     {}", total_transactions);
        eprintln!("  Total reclaimed:    {}", crate::util::disk::format_bytes(total_reclaimed));
        eprintln!();

        if !history.snapshots.is_empty() {
            let first = history.snapshots.first().unwrap();
            let last = history.snapshots.last().unwrap();
            eprintln!("  First scan:         {} (reclaimable: {})",
                chrono::DateTime::from_timestamp(first.timestamp as i64, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| first.timestamp.to_string()),
                crate::util::disk::format_bytes(first.reclaimable_bytes));
            eprintln!("  Last scan:          {} (reclaimable: {})",
                chrono::DateTime::from_timestamp(last.timestamp as i64, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| last.timestamp.to_string()),
                crate::util::disk::format_bytes(last.reclaimable_bytes));
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
            eprintln!("  {} | reclaimed {} | {} actions",
                dt,
                crate::util::disk::format_bytes(t.successful_bytes()),
                t.successful_count());
        }
    }

    Ok(())
}

