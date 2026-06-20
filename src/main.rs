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
        cli::args::Commands::Depth(args) => {
            let engine = engines::depth::DepthEngine::new(args.clone());
            vec![engine.run(ctx.clone()).await]
        }
        cli::args::Commands::All(args) => {
            run_all_engines(ctx.clone(), args).await
        }
    };

    drop(ctx);

    collector_handle.await??;

    let total_duration = scan_start.elapsed();

    if cli.global.format == OutputFormat::Report {
        let mut writer = output_writer.lock().await;
        let findings = writer.take_findings();
        let engine_stats: Vec<core::types::EngineStats> = engine_results
            .iter()
            .filter_map(|r| r.as_ref().ok().cloned())
            .collect();

        let report = core::types::ScanReport::from_findings_and_stats(
            &findings,
            &engine_stats,
            &util::macos::MacosUtils::get_macos_version(),
            util::macos::MacosUtils::is_apple_silicon(),
            total_duration,
        );
        writer.write_report(&report)?;
    } else {
        output_writer.lock().await.flush()?;
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

    if should_run(EngineIdArg::Depth) {
        let depth_engine = engines::depth::DepthEngine::default();
        results.push(depth_engine.run(ctx.clone()).await);
    }

    results
}
