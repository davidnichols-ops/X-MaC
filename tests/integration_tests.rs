use clap::Parser;
use std::path::PathBuf;
use tempfile::TempDir;
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

        let deserialized: x_mac::core::types::Finding =
            serde_json::from_str(&json).expect("Failed to deserialize");
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

        assert_eq!(
            finding.remediation_hint,
            Some("Run this command to fix".to_string())
        );
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

        assert_eq!(
            finding.metadata.get("key").unwrap(),
            &serde_json::json!("value")
        );
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
            fix_script: None,
            resource_mode: "balanced".to_string(),
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

        writer
            .write_finding(&finding)
            .expect("Failed to write finding");
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
        use x_mac::core::types::{EngineId, EngineStats, ScanReport};

        let findings = vec![
            x_mac::core::types::Finding::new(
                EngineId::Clean,
                x_mac::core::types::Severity::High,
                x_mac::core::types::Category::Cache,
                x_mac::core::types::Target::Path(PathBuf::from("/test/cache")),
                "Cache finding",
                "Test cache",
            )
            .with_size(1024 * 1024),
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
                assert!(all_args
                    .skip
                    .contains(&x_mac::cli::args::EngineIdArg::Clean));
                assert!(all_args
                    .skip
                    .contains(&x_mac::cli::args::EngineIdArg::Depth));
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

    // -- envmap engine ------------------------------------------------------

    #[test]
    fn test_cli_envmap_args_parsing() {
        let args = vec!["x-mac", "envmap"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Envmap(envmap_args) => {
                // Defaults: all discovery toggles on, redact on.
                assert!(envmap_args.apps);
                assert!(envmap_args.system);
                assert!(envmap_args.system_packages);
                assert!(envmap_args.language_packages);
                assert!(envmap_args.redact);
                assert!(!envmap_args.redact_hostnames);
            }
            _ => panic!("Expected Envmap command"),
        }
    }

    #[test]
    fn test_cli_envmap_redact_flag_off() {
        let args = vec!["x-mac", "envmap", "--redact", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Envmap(envmap_args) => {
                assert!(!envmap_args.redact);
            }
            _ => panic!("Expected Envmap command"),
        }
    }

    #[test]
    fn test_cli_envmap_redact_hostnames_flag() {
        let args = vec!["x-mac", "envmap", "--redact-hostnames", "true"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Envmap(envmap_args) => {
                assert!(envmap_args.redact_hostnames);
            }
            _ => panic!("Expected Envmap command"),
        }
    }

    #[test]
    fn test_envmap_engine_id_is_envmap() {
        let args = vec!["x-mac", "envmap"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        assert_eq!(
            cli.command.engine_id(),
            x_mac::core::types::EngineId::Envmap
        );
    }

    #[tokio::test]
    async fn test_envmap_engine_validate() {
        let engine = x_mac::engines::EnvmapEngine::default();
        let cli = x_mac::cli::args::Cli::parse_from(vec!["x-mac", "envmap"]);

        let (tx, _rx) = tokio::sync::mpsc::channel::<x_mac::core::types::Finding>(1000);
        let ctx = x_mac::core::ScanContext::new(&cli, tx).await.unwrap();
        let result = engine.validate(&ctx).await;
        assert!(result.is_ok());
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_envmap_engine_scan_emits_findings() {
        // Use a temp dir as an app dir so the apps scanner has something to
        // walk without touching the real /Applications.
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let app_dir = tmp.path().join("MyApp.app");
        std::fs::create_dir_all(app_dir.join("Contents")).unwrap();
        std::fs::write(
            app_dir.join("Contents").join("Info.plist"),
            r#"<plist version="1.0"><dict>
                <key>CFBundleName</key><string>MyApp</string>
                <key>CFBundleShortVersionString</key><string>1.0</string>
            </dict></plist>"#,
        )
        .unwrap();

        let engine = x_mac::engines::EnvmapEngine::new(x_mac::cli::args::EnvmapArgs {
            system: true,
            system_packages: false,
            language_packages: false,
            apps: true,
            app_dirs: vec![tmp.path().to_path_buf()],
            redact: true,
            redact_hostnames: false,
        });

        let cli = x_mac::cli::args::Cli::parse_from(vec!["x-mac", "envmap"]);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<x_mac::core::types::Finding>(1000);
        let ctx = std::sync::Arc::new(x_mac::core::ScanContext::new(&cli, tx).await.unwrap());

        let stats = engine.scan(ctx).await.expect("scan should succeed");
        assert!(stats.findings_count >= 1);
        assert_eq!(stats.engine, x_mac::core::types::EngineId::Envmap);

        // Drain emitted findings and confirm at least one is an InstalledApp.
        let mut found_app = false;
        while let Ok(f) = rx.try_recv() {
            if f.category == x_mac::core::types::Category::InstalledApp {
                found_app = true;
                // Redaction is on — the temp path is under /private/var, not
                // /Users, so the bundle_name should still be present.
                assert!(f.title.contains("MyApp"));
            }
        }
        assert!(found_app, "expected an InstalledApp finding");
    }

    #[test]
    fn test_envmap_redactor_default_rule_count() {
        let r = x_mac::engines::envmap::redaction::Redactor::new();
        assert_eq!(r.rule_count(), 17);
    }

    #[test]
    fn test_envmap_redactor_disabled_is_noop() {
        let r = x_mac::engines::envmap::redaction::Redactor::disabled();
        assert_eq!(
            r.redact("/Users/alice/secret=hunter2"),
            "/Users/alice/secret=hunter2"
        );
    }

    #[test]
    fn test_envmap_report_includes_envmap_breakdown() {
        use x_mac::core::types::{
            EngineBreakdown, EngineId, EngineStats, Finding, ScanReport, Severity,
        };

        let findings = vec![Finding::new(
            EngineId::Envmap,
            Severity::Info,
            x_mac::core::types::Category::InstalledApp,
            x_mac::core::types::Target::Path(PathBuf::from("/Applications/X.app")),
            "App: X 1.0",
            "Installed app",
        )];

        let engine_stats = vec![EngineStats {
            engine: EngineId::Envmap,
            duration: std::time::Duration::from_millis(10),
            items_scanned: 1,
            findings_count: 1,
            errors_count: 0,
        }];

        let report = ScanReport::from_findings_and_stats(
            &findings,
            &engine_stats,
            "14.5.0",
            true,
            std::time::Duration::from_millis(10),
        );

        assert_eq!(report.findings_by_engine.envmap, 1);
        // Ensure the new field serializes.
        let json = serde_json::to_string(&report).expect("serialize");
        assert!(json.contains("envmap"));
        let _bd: EngineBreakdown =
            serde_json::from_str(&serde_json::to_string(&report.findings_by_engine).unwrap())
                .expect("deserialize breakdown");
    }

    #[test]
    fn test_scan_skip_supports_envmap() {
        let args = vec!["x-mac", "scan", "--skip", "envmap"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Scan(scan_args) => {
                assert!(scan_args
                    .skip
                    .contains(&x_mac::cli::args::ScanEngineIdArg::Envmap));
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_all_skip_supports_envmap() {
        let args = vec!["x-mac", "all", "--skip", "envmap"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::All(all_args) => {
                assert!(all_args
                    .skip
                    .contains(&x_mac::cli::args::EngineIdArg::Envmap));
            }
            _ => panic!("Expected All command"),
        }
    }

    // -- new clean categories: temp, build artifacts, pkg caches -----------

    #[test]
    fn test_cli_clean_new_flags_default_true() {
        let args = vec!["x-mac", "clean"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Clean(clean_args) => {
                assert!(clean_args.pkg_caches);
                assert!(clean_args.temp);
                assert!(clean_args.build_artifacts);
            }
            _ => panic!("Expected Clean command"),
        }
    }

    #[test]
    fn test_cli_clean_disable_build_artifacts() {
        let args = vec!["x-mac", "clean", "--build-artifacts", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Clean(clean_args) => {
                assert!(!clean_args.build_artifacts);
            }
            _ => panic!("Expected Clean command"),
        }
    }

    #[test]
    fn test_cli_clean_disable_temp() {
        let args = vec!["x-mac", "clean", "--temp", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Clean(clean_args) => {
                assert!(!clean_args.temp);
            }
            _ => panic!("Expected Clean command"),
        }
    }

    #[test]
    fn test_cli_clean_disable_pkg_caches() {
        let args = vec!["x-mac", "clean", "--pkg-caches", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);

        match cli.command {
            x_mac::cli::args::Commands::Clean(clean_args) => {
                assert!(!clean_args.pkg_caches);
            }
            _ => panic!("Expected Clean command"),
        }
    }

    #[test]
    fn test_new_category_serialization() {
        for category in [
            x_mac::core::types::Category::TempFile,
            x_mac::core::types::Category::BuildArtifact,
            x_mac::core::types::Category::PackageManagerCache,
        ] {
            let finding = x_mac::core::types::Finding::new(
                x_mac::core::types::EngineId::Clean,
                x_mac::core::types::Severity::Low,
                category,
                x_mac::core::types::Target::Path(PathBuf::from("/test")),
                "Test",
                "desc",
            );
            let json = serde_json::to_string(&finding).expect("serialize");
            let de: x_mac::core::types::Finding = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(de.category, category);
        }
    }

    #[test]
    fn test_fix_script_handles_build_artifact() {
        use x_mac::cli::fix_script::FixScriptGenerator;
        let f = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Medium,
            x_mac::core::types::Category::BuildArtifact,
            x_mac::core::types::Target::Path(PathBuf::from("/home/proj/node_modules")),
            "Build artifact directory: node_modules",
            "desc",
        );
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("# rm -rf -- '/home/proj/node_modules'"));
        assert!(script.contains("Build artifacts"));
    }

    #[test]
    fn test_fix_script_handles_pkg_cache() {
        use x_mac::cli::fix_script::FixScriptGenerator;
        let f = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Low,
            x_mac::core::types::Category::PackageManagerCache,
            x_mac::core::types::Target::Path(PathBuf::from("/home/.npm/_cacache")),
            "Package-manager cache detected",
            "desc",
        );
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("# rm -rf -- '/home/.npm/_cacache'"));
        assert!(script.contains("Package-manager caches"));
    }

    #[test]
    fn test_fix_script_handles_temp_file() {
        use x_mac::cli::fix_script::FixScriptGenerator;
        let f = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Low,
            x_mac::core::types::Category::TempFile,
            x_mac::core::types::Target::Path(PathBuf::from("/private/tmp")),
            "System temp directory",
            "desc",
        );
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("# rm -rf -- '/private/tmp'"));
        assert!(script.contains("Temp files"));
    }

    #[test]
    fn test_clean_engine_default_has_new_flags() {
        let engine = x_mac::engines::CleanEngine::default();
        // Just validate it constructs — the flags are exercised at scan time.
        assert_eq!(engine.id(), x_mac::core::types::EngineId::Clean);
    }

    #[test]
    fn test_build_artifact_dirs_constant() {
        use x_mac::engines::clean::rules::BUILD_ARTIFACT_DIRS;
        assert!(BUILD_ARTIFACT_DIRS.contains(&"node_modules"));
        assert!(BUILD_ARTIFACT_DIRS.contains(&"target"));
        assert!(BUILD_ARTIFACT_DIRS.contains(&"__pycache__"));
    }

    #[test]
    fn test_build_artifact_file_patterns_constant() {
        use x_mac::engines::clean::rules::BUILD_ARTIFACT_FILE_PATTERNS;
        assert!(BUILD_ARTIFACT_FILE_PATTERNS.contains(&".pyc"));
        assert!(BUILD_ARTIFACT_FILE_PATTERNS.contains(&".o"));
    }

    #[test]
    fn test_rotated_log_extensions_constant() {
        use x_mac::engines::clean::rules::ROTATED_LOG_EXTENSIONS;
        assert!(ROTATED_LOG_EXTENSIONS.contains(&".gz"));
        assert!(ROTATED_LOG_EXTENSIONS.contains(&".bz2"));
    }

    // -- new clean categories: browser, mail, iOS, languages, trash, large --

    #[test]
    fn test_cli_clean_browser_flag() {
        let args = vec!["x-mac", "clean", "--browser", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Clean(c) => assert!(!c.browser),
            _ => panic!("Expected Clean"),
        }
    }

    #[test]
    fn test_cli_clean_mail_flag() {
        let args = vec!["x-mac", "clean", "--mail", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Clean(c) => assert!(!c.mail),
            _ => panic!("Expected Clean"),
        }
    }

    #[test]
    fn test_cli_clean_ios_backups_flag() {
        let args = vec!["x-mac", "clean", "--ios-backups", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Clean(c) => assert!(!c.ios_backups),
            _ => panic!("Expected Clean"),
        }
    }

    #[test]
    fn test_cli_clean_languages_flag() {
        let args = vec!["x-mac", "clean", "--languages", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Clean(c) => assert!(!c.languages),
            _ => panic!("Expected Clean"),
        }
    }

    #[test]
    fn test_cli_clean_trash_flag() {
        let args = vec!["x-mac", "clean", "--trash", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Clean(c) => assert!(!c.trash),
            _ => panic!("Expected Clean"),
        }
    }

    #[test]
    fn test_cli_clean_large_files_flag() {
        let args = vec!["x-mac", "clean", "--large-files", "false"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Clean(c) => assert!(!c.large_files),
            _ => panic!("Expected Clean"),
        }
    }

    #[test]
    fn test_cli_clean_min_large_size() {
        let args = vec!["x-mac", "clean", "--min-large-size", "500M"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Clean(c) => assert_eq!(c.min_large_size, "500M"),
            _ => panic!("Expected Clean"),
        }
    }

    #[test]
    fn test_new_clean_categories_serialize() {
        for category in [
            x_mac::core::types::Category::BrowserCache,
            x_mac::core::types::Category::MailAttachment,
            x_mac::core::types::Category::IosBackup,
            x_mac::core::types::Category::LanguageFile,
            x_mac::core::types::Category::UniversalBinary,
            x_mac::core::types::Category::LargeFile,
            x_mac::core::types::Category::TrashBin,
            x_mac::core::types::Category::DocumentVersion,
            x_mac::core::types::Category::SystemMaintenance,
        ] {
            let f = x_mac::core::types::Finding::new(
                x_mac::core::types::EngineId::Clean,
                x_mac::core::types::Severity::Low,
                category,
                x_mac::core::types::Target::Path(PathBuf::from("/test")),
                "Test",
                "desc",
            );
            let json = serde_json::to_string(&f).expect("serialize");
            let de: x_mac::core::types::Finding = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(de.category, category);
        }
    }

    // -- maintain engine ----------------------------------------------------

    #[test]
    fn test_cli_quick_args_parsing() {
        let args = vec!["x-mac", "quick"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Quick(q) => {
                assert!(!q.dedup);
                assert!(!q.no_maintain);
                assert!(!q.no_disk);
            }
            _ => panic!("Expected Quick"),
        }
    }

    #[test]
    fn test_cli_quick_with_options() {
        let args = vec!["x-mac", "quick", "--dedup", "--no-maintain"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Quick(q) => {
                assert!(q.dedup);
                assert!(q.no_maintain);
                assert!(!q.no_disk);
            }
            _ => panic!("Expected Quick"),
        }
    }

    #[test]
    fn test_cli_doctor_alias() {
        let args = vec!["x-mac", "doctor"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Doctor(_) => {}
            _ => panic!("Expected Doctor"),
        }
    }

    #[test]
    fn test_default_format_is_report() {
        let args = vec!["x-mac", "quick"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        assert_eq!(cli.global.format, x_mac::cli::args::OutputFormat::Report);
    }

    #[test]
    fn test_cli_maintain_args_parsing() {
        let args = vec!["x-mac", "maintain"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Maintain(m) => {
                assert!(m.dns);
                assert!(m.spotlight);
                assert!(m.launchservices);
                assert!(m.periodic);
                assert!(!m.repair_permissions);
                assert!(m.purge_ram);
                assert!(!m.dyld);
                assert!(m.quicklook);
            }
            _ => panic!("Expected Maintain"),
        }
    }

    #[test]
    fn test_cli_maintain_selective() {
        let args = vec![
            "x-mac",
            "maintain",
            "--spotlight",
            "false",
            "--periodic",
            "false",
        ];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Maintain(m) => {
                assert!(m.dns);
                assert!(!m.spotlight);
                assert!(!m.periodic);
            }
            _ => panic!("Expected Maintain"),
        }
    }

    #[tokio::test]
    async fn test_maintain_engine_validate() {
        let engine = x_mac::engines::MaintainEngine::default();
        let cli = x_mac::cli::args::Cli::parse_from(vec!["x-mac", "maintain"]);
        let (tx, _rx) = tokio::sync::mpsc::channel::<x_mac::core::types::Finding>(1000);
        let ctx = x_mac::core::ScanContext::new(&cli, tx).await.unwrap();
        let result = engine.validate(&ctx).await;
        assert!(result.is_ok());
    }

    // -- disk engine --------------------------------------------------------

    #[test]
    fn test_cli_disk_args_parsing() {
        let args = vec!["x-mac", "disk", "--top", "10", "--min-size", "50M"];
        let cli = x_mac::cli::args::Cli::parse_from(args);
        match cli.command {
            x_mac::cli::args::Commands::Disk(d) => {
                assert_eq!(d.top, 10);
                assert_eq!(d.min_size, "50M");
            }
            _ => panic!("Expected Disk"),
        }
    }

    #[tokio::test]
    async fn test_disk_engine_validate() {
        let engine = x_mac::engines::DiskEngine::default();
        let cli = x_mac::cli::args::Cli::parse_from(vec!["x-mac", "disk"]);
        let (tx, _rx) = tokio::sync::mpsc::channel::<x_mac::core::types::Finding>(1000);
        let ctx = x_mac::core::ScanContext::new(&cli, tx).await.unwrap();
        let result = engine.validate(&ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_disk_engine_scan_temp_dir() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        // Create a file larger than the default 100M threshold would be
        // impractical in tests, so use a low threshold via args.
        let big_file = tmp.path().join("big.bin");
        std::fs::write(&big_file, vec![0u8; 1024]).unwrap();

        let engine = x_mac::engines::DiskEngine::new(x_mac::cli::args::DiskArgs {
            top: 10,
            min_size: "100B".to_string(),
            paths: vec![tmp.path().to_path_buf()],
        });

        let cli = x_mac::cli::args::Cli::parse_from(vec!["x-mac", "disk"]);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<x_mac::core::types::Finding>(1000);
        let ctx = std::sync::Arc::new(x_mac::core::ScanContext::new(&cli, tx).await.unwrap());

        let stats = engine.scan(ctx).await.expect("scan should succeed");
        assert!(stats.findings_count >= 1);

        let mut found = false;
        while let Ok(f) = rx.try_recv() {
            if f.title.contains("big.bin") {
                found = true;
            }
        }
        assert!(found, "expected to find big.bin in disk scan results");
    }

    // -- fix script with new categories -------------------------------------

    #[test]
    fn test_fix_script_handles_browser_cache() {
        use x_mac::cli::fix_script::FixScriptGenerator;
        let f = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Low,
            x_mac::core::types::Category::BrowserCache,
            x_mac::core::types::Target::Path(PathBuf::from("/home/Library/Caches/Google/Chrome")),
            "Chrome browser cache",
            "desc",
        );
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("# rm -rf -- '/home/Library/Caches/Google/Chrome'"));
        assert!(script.contains("Browser caches"));
    }

    #[test]
    fn test_fix_script_handles_trash_bin() {
        use x_mac::cli::fix_script::FixScriptGenerator;
        let f = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Medium,
            x_mac::core::types::Category::TrashBin,
            x_mac::core::types::Target::Path(PathBuf::from("/home/.Trash")),
            "Trash bin with content",
            "desc",
        );
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("# rm -rf -- '/home/.Trash'"));
        assert!(script.contains("Trash bins"));
    }

    #[test]
    fn test_fix_script_handles_ios_backup() {
        use x_mac::cli::fix_script::FixScriptGenerator;
        let f = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Medium,
            x_mac::core::types::Category::IosBackup,
            x_mac::core::types::Target::Path(PathBuf::from(
                "/home/Library/Application Support/MobileSync/Backup/abc123",
            )),
            "iOS device backup detected",
            "desc",
        );
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("# rm -rf"));
        assert!(script.contains("iOS device backups"));
    }

    #[test]
    fn test_fix_script_handles_system_maintenance() {
        use x_mac::cli::fix_script::FixScriptGenerator;
        let f = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::All,
            x_mac::core::types::Severity::Info,
            x_mac::core::types::Category::SystemMaintenance,
            x_mac::core::types::Target::Path(PathBuf::from("/")),
            "DNS cache flush",
            "desc",
        )
        .with_hint("dscacheutil -flushcache; killall -HUP mDNSResponder");
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("dscacheutil -flushcache"));
        assert!(script.contains("System maintenance tasks"));
    }

    #[test]
    fn test_fix_script_large_file_is_informational() {
        use x_mac::cli::fix_script::FixScriptGenerator;
        let f = x_mac::core::types::Finding::new(
            x_mac::core::types::EngineId::Clean,
            x_mac::core::types::Severity::Low,
            x_mac::core::types::Category::LargeFile,
            x_mac::core::types::Target::Path(PathBuf::from("/home/big.iso")),
            "Large file: big.iso",
            "desc",
        );
        let script = FixScriptGenerator::build_script(&[f]);
        // Large files are informational — should not have rm commands.
        assert!(!script.contains("rm -rf"));
        // The finding is filtered out entirely (no fix to apply), so it
        // shouldn't appear as a review-required fix either.
        assert!(!script.contains("big.iso"));
    }
}
