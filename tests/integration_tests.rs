use std::path::PathBuf;
use tempfile::TempDir;
use clap::Parser;
use x_mac::core::engine::Engine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finding_serialization() {
        let finding = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Low,
            x_mac::core::types::Category::Cache,
            x_mac::core::types::Target::Path(PathBuf::from("/test/path")),
            "Test finding",
            "Test description",
        );

        let json = serde_json::to_string(&finding).expect("Failed to serialize finding");
        assert!(json.contains("Test finding"));

        let deserialized: x_mac::core::types::Finding = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.title, "Test finding");
    }

    #[test]
    fn test_finding_with_size() {
        let finding = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Low,
            x_mac::core::types::Category::Cache,
            x_mac::core::types::Target::Path(PathBuf::from("/test/path")),
            "Test finding",
            "Test description",
        )
        .with_size(1024);

        assert_eq!(finding.size_bytes, Some(1024));
    }

    #[test]
    fn test_finding_with_hint() {
        let finding = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Low,
            x_mac::core::types::Category::Cache,
            x_mac::core::types::Target::Path(PathBuf::from("/test/path")),
            "Test finding",
            "Test description",
        )
        .with_hint("Run this command to fix");

        assert_eq!(finding.remediation_hint, Some("Run this command to fix".to_string()));
    }

    #[test]
    fn test_finding_with_metadata() {
        let finding = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Low,
            x_mac::core::types::Category::Cache,
            x_mac::core::types::Target::Path(PathBuf::from("/test/path")),
            "Test finding",
            "Test description",
        )
        .with_metadata("key", serde_json::json!("value"));

        assert_eq!(finding.metadata.get("key").unwrap(), &serde_json::json!("value"));
    }

    #[test]
    fn test_cli_clean_args_parsing() {
        let args = vec!["x-mac", "clean", "--min-age", "7d", "--min-size", "100k"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Clean(clean_args) => {
                assert_eq!(clean_args.min_age, "7d");
                assert_eq!(clean_args.min_size, "100k");
            }
            _ => panic!("Expected Clean command"),
        }
    }

    #[test]
    fn test_cli_conflict_args_parsing() {
        let args = vec!["x-mac", "conflict", "--path", "--ports"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Conflict(conflict_args) => {
                assert!(conflict_args.path);
                assert!(conflict_args.ports);
            }
            _ => panic!("Expected Conflict command"),
        }
    }

    #[test]
    fn test_cli_map_args_parsing() {
        let args = vec!["x-mac", "map", "--python", "--nodejs"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Map(map_args) => {
                assert!(map_args.python);
                assert!(map_args.nodejs);
            }
            _ => panic!("Expected Map command"),
        }
    }

    #[test]
    fn test_cli_depth_args_parsing() {
        let args = vec!["x-mac", "depth", "--permissions", "--symlinks"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Depth(depth_args) => {
                assert!(depth_args.permissions);
                assert!(depth_args.symlinks);
            }
            _ => panic!("Expected Depth command"),
        }
    }

    #[test]
    fn test_output_writer_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let global_args = x_mac::cli::args::GlobalArgs {
            format: x_mac::cli::args::OutputFormat::Json,
            output: Some(output_path.clone()),
            verbose: 0,
            quiet: true,
            concurrency: 4,
            exclude: vec![],
            include_hidden: false,
            follow_symlinks: false,
            cache_dir: None,
        };

        let mut writer = x_mac::cli::output::OutputWriter::new(&global_args);

        let finding = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Low,
            x_mac::core::types::Category::Cache,
            x_mac::core::types::Target::Path(PathBuf::from("/test/path")),
            "Test finding",
            "Test description",
        );

        writer.write_finding(&finding).expect("Failed to write finding");
        writer.flush().expect("Failed to flush");

        let content = std::fs::read_to_string(&output_path).expect("Failed to read output");
        assert!(content.contains("Test finding"));
    }

    #[test]
    fn test_engine_id_ordering() {
        assert!(x_mac::core::types::Severity::Info < x_mac::core::types::Severity::Low);
        assert!(x_mac::core::types::Severity::Low < x_mac::core::types::Severity::Medium);
        assert!(x_mac::core::types::Severity::Medium < x_mac::core::types::Severity::High);
        assert!(x_mac::core::types::Severity::High < x_mac::core::types::Severity::Critical);
    }

    #[test]
    fn test_scan_config_default() {
        let config = x_mac::core::types::ScanConfig::default();
        assert_eq!(config.concurrency, 4);
        assert!(!config.include_hidden);
        assert!(!config.follow_symlinks);
        assert!(config.exclude_patterns.is_empty());
        assert!(config.cache_dir.is_none());
    }

    #[tokio::test]
    async fn test_clean_engine_validate() {
        let engine = x_mac::engines::CleanEngine::default();
        let cli = x_mac::cli::args::Cli::parse_from(vec!["x-mac", "clean"]);

        let (tx, _rx) = tokio::sync::mpsc::channel::<x_mac::core::types::Finding>(1000);
        let ctx = x_mac::core::ScanContext::new(&cli, tx).await.unwrap();
        let result = engine.validate(&ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_conflict_engine_validate() {
        let engine = x_mac::engines::ConflictEngine::default();
        let cli = x_mac::cli::args::Cli::parse_from(vec!["x-mac", "conflict"]);

        let (tx, _rx) = tokio::sync::mpsc::channel::<x_mac::core::types::Finding>(1000);
        let ctx = x_mac::core::ScanContext::new(&cli, tx).await.unwrap();
        let result = engine.validate(&ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_map_engine_validate() {
        let engine = x_mac::engines::MapEngine::default();
        let cli = x_mac::cli::args::Cli::parse_from(vec!["x-mac", "map"]);

        let (tx, _rx) = tokio::sync::mpsc::channel::<x_mac::core::types::Finding>(1000);
        let ctx = x_mac::core::ScanContext::new(&cli, tx).await.unwrap();
        let result = engine.validate(&ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_depth_engine_validate() {
        let engine = x_mac::engines::DepthEngine::default();
        let cli = x_mac::cli::args::Cli::parse_from(vec!["x-mac", "depth"]);

        let (tx, _rx) = tokio::sync::mpsc::channel::<x_mac::core::types::Finding>(1000);
        let ctx = x_mac::core::ScanContext::new(&cli, tx).await.unwrap();
        let result = engine.validate(&ctx).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_macos_utils() {
        let home = x_mac::util::MacosUtils::home_dir();
        assert!(home.to_string_lossy().contains('/'));

        let is_silicon = x_mac::util::MacosUtils::is_apple_silicon();
        assert!(matches!(is_silicon, true | false));

        let version = x_mac::util::MacosUtils::get_macos_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_disk_utilities() {
        let formatted = x_mac::util::disk::format_bytes(1024);
        assert!(formatted.contains("KB"));

        let formatted_gb = x_mac::util::disk::format_bytes(1024 * 1024 * 1024);
        assert!(formatted_gb.contains("GB"));
    }

    #[test]
    fn test_scan_report_serialization() {
        use x_mac::core::types::{ScanReport, EngineStats, EngineId};

        let findings = vec![
            x_mac::core::types::Finding::new(
                EngineId::Clean,
                x_mac::core::types::Severity::High,
                x_mac::core::types::Category::Cache,
                x_mac::core::types::Target::Path(PathBuf::from("/test/cache")),
                "Cache finding",
                "Test cache",
            ).with_size(1024 * 1024),
            x_mac::core::types::Finding::new(
                EngineId::Conflict,
                x_mac::core::types::Severity::Low,
                x_mac::core::types::Category::PortConflict,
                x_mac::core::types::Target::Port(3000),
                "Port finding",
                "Test port",
            ),
        ];

        let engine_stats = vec![
            EngineStats {
                engine: EngineId::Clean,
                duration: std::time::Duration::from_millis(100),
                items_scanned: 50,
                findings_count: 1,
                errors_count: 0,
            },
            EngineStats {
                engine: EngineId::Conflict,
                duration: std::time::Duration::from_millis(50),
                items_scanned: 10,
                findings_count: 1,
                errors_count: 0,
            },
        ];

        let report = ScanReport::from_findings_and_stats(
            &findings,
            &engine_stats,
            "14.5.0",
            true,
            std::time::Duration::from_millis(150),
        );

        assert_eq!(report.total_findings, 2);
        assert_eq!(report.findings_by_severity.high, 1);
        assert_eq!(report.findings_by_severity.low, 1);
        assert_eq!(report.findings_by_engine.clean, 1);
        assert_eq!(report.findings_by_engine.conflict, 1);
        assert_eq!(report.total_reclaimable_bytes, 1024 * 1024);
        assert_eq!(report.total_items_scanned, 60);
        assert_eq!(report.macos_version, "14.5.0");
        assert!(report.apple_silicon);

        let json = serde_json::to_string(&report).expect("Failed to serialize report");
        assert!(json.contains("total_findings"));
        assert!(json.contains("findings_by_severity"));
        assert!(json.contains("total_reclaimable_bytes"));
    }

    #[test]
    fn test_report_output_format() {
        let args = vec!["x-mac", "--format", "report", "clean"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        assert_eq!(cli.global.format, x_mac::cli::args::OutputFormat::Report);
    }

    #[test]
    fn test_all_args_skip_parsing() {
        let args = vec!["x-mac", "all", "--skip", "clean", "--skip", "depth"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::All(all_args) => {
                assert_eq!(all_args.skip.len(), 2);
                assert!(all_args.skip.contains(&x_mac::cli::args::EngineIdArg::Clean));
                assert!(all_args.skip.contains(&x_mac::cli::args::EngineIdArg::Depth));
            }
            _ => panic!("Expected All command"),
        }
    }

    #[test]
    fn test_engine_id_all_variant() {
        let args = vec!["x-mac", "all"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        assert_eq!(cli.command.engine_id(), x_mac::core::types::EngineId::All);
    }
}
