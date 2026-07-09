use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn};

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
        cli::args::Commands::Scan(args) => {
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
        cli::args::Commands::All(args) => {
            run_all_engines(ctx.clone(), args).await
        }
        cli::args::Commands::Install(args) => {
            // Handle install before the scan pipeline — it doesn't scan.
            return run_install(&cli, args);
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
            // Default: /opt/homebrew/bin on Apple Silicon, /usr/local/bin on Intel.
            if util::macos::MacosUtils::is_apple_silicon() {
                std::path::PathBuf::from("/opt/homebrew/bin")
            } else {
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
