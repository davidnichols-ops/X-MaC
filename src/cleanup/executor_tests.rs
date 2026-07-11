#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::cleanup::{CleanupExecutor, CleanupPolicy};
    use crate::core::types::{Category, EngineId, Finding, Severity, Target};

    fn make_finding(path: impl Into<PathBuf>, category: Category, size: u64) -> Finding {
        Finding::new(
            EngineId::Clean,
            Severity::Low,
            category,
            Target::Path(path.into()),
            "Test finding",
            "Test description",
        )
        .with_size(size)
    }

    fn test_home() -> PathBuf {
        crate::util::macos::MacosUtils::home_dir()
            .join(".xmac_test_cleanup")
    }

    fn setup_test_dir(name: &str) -> PathBuf {
        let dir = test_home().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup_test_dir(name: &str) {
        let _ = std::fs::remove_dir_all(test_home().join(name));
    }

    #[test]
    fn trash_safe_cache_file() {
        // The cleanup engine only moves items inside the user's home directory.
        let home = setup_test_dir("cache");
        let file = home.join("cache.bin");
        std::fs::write(&file, b"data").unwrap();

        let findings = vec![make_finding(&file, Category::Cache, 4)];
        let executor = CleanupExecutor::new(CleanupPolicy::safe(), false);
        let plan = executor.plan(&findings);
        let mut executor = executor;
        let transaction = executor.execute(&plan);

        for action in &transaction.actions {
            if !action.success {
                eprintln!("action failed: {:?} -> error: {:?}", action, action.error);
            }
        }
        assert!(!file.exists(), "file still exists: {:?}", file);
        assert_eq!(transaction.successful_count(), 1);
        cleanup_test_dir("cache");
    }

    #[test]
    fn dry_run_does_not_delete() {
        let home = setup_test_dir("dry_run");
        let file = home.join("cache.bin");
        std::fs::write(&file, b"data").unwrap();

        let findings = vec![make_finding(&file, Category::Cache, 4)];
        let executor = CleanupExecutor::new(CleanupPolicy::safe(), true);
        let plan = executor.plan(&findings);
        let mut executor = executor;
        let transaction = executor.execute(&plan);

        assert!(file.exists());
        assert_eq!(transaction.successful_count(), 1); // dry-run records success
        cleanup_test_dir("dry_run");
    }

    #[test]
    fn system_path_is_blocked() {
        let findings = vec![make_finding("/Applications/SomeApp.app", Category::Cache, 100)];
        let executor = CleanupExecutor::new(CleanupPolicy::safe(), false);
        let plan = executor.plan(&findings);
        assert!(plan.executable().is_empty());
    }

    #[test]
    fn large_file_is_review_not_trash() {
        let home = setup_test_dir("large_file");
        let file = home.join("large.bin");
        std::fs::write(&file, b"data").unwrap();

        let findings = vec![make_finding(&file, Category::LargeFile, 100)];
        let executor = CleanupExecutor::new(CleanupPolicy::safe(), false);
        let plan = executor.plan(&findings);
        let mut executor = executor;
        let transaction = executor.execute(&plan);

        assert!(file.exists());
        assert_eq!(transaction.successful_count(), 0);
        cleanup_test_dir("large_file");
    }
}
