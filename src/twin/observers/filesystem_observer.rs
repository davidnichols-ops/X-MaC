// FilesystemObserver — real-time file change monitoring via FSEvents
//
// Uses the `notify` crate (which uses FSEvents on macOS, inotify on Linux)
// to watch for file system changes in real-time. Emits events:
//   - FileCreated: new files created
//   - FileDeleted: files removed
//   - LargeFileDetected: files created that are >100MB
//
// Watches the user's home directory by default (excluding caches and
// system paths that generate too much noise).

use crate::twin::database::event_store::{EventStore, StoredEvent};
use anyhow::Result;
use notify::{Config, Event as NotifyEvent, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// Configuration for the filesystem observer.
#[derive(Debug, Clone)]
pub struct FilesystemObserverConfig {
    /// Directories to watch (default: user home dir).
    pub watch_paths: Vec<PathBuf>,
    /// File size threshold for LargeFileDetected (default: 100 MB).
    pub large_file_threshold: u64,
    /// Paths to exclude (substring match).
    pub exclude_paths: Vec<String>,
    /// Maximum events to buffer before dropping.
    pub max_buffer: usize,
}

impl Default for FilesystemObserverConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        Self {
            watch_paths: vec![home],
            large_file_threshold: 100 * 1024 * 1024,
            exclude_paths: vec![
                "/.Trash/".to_string(),
                "/Library/Caches/".to_string(),
                "/.cache/".to_string(),
                "/node_modules/".to_string(),
                "/.git/".to_string(),
                "/target/".to_string(),
                "/.npm/".to_string(),
                "/Library/Application Support/".to_string(),
            ],
            max_buffer: 10000,
        }
    }
}

/// The filesystem observer. Wraps a notify watcher and drains events.
pub struct FilesystemObserver {
    config: FilesystemObserverConfig,
    receiver: Receiver<NotifyEvent>,
    _watcher: RecommendedWatcher,
}

impl FilesystemObserver {
    pub fn new(config: FilesystemObserverConfig) -> Result<Self> {
        let (tx, rx) = channel::<NotifyEvent>();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<NotifyEvent>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;

        for path in &config.watch_paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)?;
            }
        }

        Ok(Self {
            config,
            receiver: rx,
            _watcher: watcher,
        })
    }

    pub fn with_defaults() -> Result<Self> {
        Self::new(FilesystemObserverConfig::default())
    }

    /// Check if a path should be excluded.
    fn is_excluded(&self, path: &str) -> bool {
        for exclude in &self.config.exclude_paths {
            if path.contains(exclude) {
                return true;
            }
        }
        false
    }

    /// Get file size if the path exists.
    fn file_size(path: &Path) -> u64 {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    }

    /// Drain pending events from the watcher and convert to StoredEvents.
    /// Returns events ready to be appended to the EventStore.
    pub fn drain_events(&self) -> Vec<StoredEvent> {
        let mut events = Vec::new();
        let now_ms = chrono::Utc::now().timestamp_millis();

        while let Ok(notify_event) = self.receiver.try_recv() {
            if events.len() >= self.config.max_buffer {
                break;
            }

            let kind = &notify_event.kind;
            for path in &notify_event.paths {
                let path_str = path.display().to_string();

                // Skip excluded paths.
                if self.is_excluded(&path_str) {
                    continue;
                }

                let (event_type, severity) = match kind {
                    EventKind::Create(_) => ("FileCreated", "info"),
                    EventKind::Remove(_) => ("FileDeleted", "info"),
                    EventKind::Modify(_) => {
                        // Skip modifications — too noisy.
                        continue;
                    }
                    EventKind::Access(_) | EventKind::Any | EventKind::Other => {
                        continue;
                    }
                };

                let size = Self::file_size(path);

                // Check for large file on create.
                let (final_event_type, final_severity) =
                    if event_type == "FileCreated" && size > self.config.large_file_threshold {
                        ("LargeFileDetected", "warning")
                    } else {
                        (event_type, severity)
                    };

                let mut payload = std::collections::HashMap::new();
                payload.insert("path".to_string(), serde_json::json!(path_str));
                payload.insert("size".to_string(), serde_json::json!(size));

                // Add file name.
                if let Some(name) = path.file_name() {
                    payload.insert(
                        "name".to_string(),
                        serde_json::json!(name.to_string_lossy().to_string()),
                    );
                }

                events.push(StoredEvent {
                    id: String::new(),
                    timestamp_ms: now_ms,
                    event_type: final_event_type.to_string(),
                    severity: final_severity.to_string(),
                    source: "filesystem_observer".to_string(),
                    entity_id: Some(format!("file:{}", path_str)),
                    payload,
                });
            }
        }

        events
    }

    /// Drain and append all pending events to the store.
    /// Returns the number of events appended.
    pub async fn drain_and_store(&self, store: &EventStore) -> Result<usize> {
        let events = self.drain_events();
        if events.is_empty() {
            return Ok(0);
        }
        store.append_batch(events).await
    }

    /// Check if there are pending events.
    pub fn has_pending(&self) -> bool {
        // Non-blocking check — if try_recv succeeds, there's at least one.
        // We don't consume it here; drain_events will.
        self.receiver.try_recv().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twin::database::TwinDb;
    use std::io::Write;

    #[tokio::test]
    async fn test_filesystem_observer_creates_and_deletes() {
        let db = TwinDb::open_memory().unwrap();
        let store = EventStore::new(db.handle());

        // Watch a temp directory.
        let tmp = std::env::temp_dir().join("xmac_fs_observer_test");
        std::fs::create_dir_all(&tmp).unwrap();

        let config = FilesystemObserverConfig {
            watch_paths: vec![tmp.clone()],
            exclude_paths: vec![], // no excludes for the test
            ..Default::default()
        };
        let observer = FilesystemObserver::new(config).unwrap();

        // Create a file.
        let test_file = tmp.join("test_create.txt");
        std::fs::write(&test_file, "hello world").unwrap();

        // Give the watcher a moment to notice.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Drain events.
        let events = observer.drain_events();
        assert!(
            !events.is_empty(),
            "should detect file creation, got {} events",
            events.len()
        );
        let has_create = events.iter().any(|e| e.event_type == "FileCreated");
        assert!(has_create, "should have a FileCreated event");

        // Append to store.
        let count = store.append_batch(events).await.unwrap();
        assert!(count > 0);

        // Delete the file.
        std::fs::remove_file(&test_file).unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;

        let events2 = observer.drain_events();
        let has_delete = events2.iter().any(|e| e.event_type == "FileDeleted");
        assert!(has_delete, "should have a FileDeleted event");

        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn test_large_file_detection() {
        let db = TwinDb::open_memory().unwrap();
        let _store = EventStore::new(db.handle());

        let tmp = std::env::temp_dir().join("xmac_fs_large_test");
        std::fs::create_dir_all(&tmp).unwrap();

        let config = FilesystemObserverConfig {
            watch_paths: vec![tmp.clone()],
            exclude_paths: vec![],
            large_file_threshold: 1024, // 1KB threshold for testing
            ..Default::default()
        };
        let observer = FilesystemObserver::new(config).unwrap();

        // Create a "large" file (> 1KB threshold).
        let large_file = tmp.join("large.bin");
        let mut f = std::fs::File::create(&large_file).unwrap();
        let buf = vec![0u8; 2048];
        f.write_all(&buf).unwrap();
        f.sync_all().unwrap();
        drop(f);

        tokio::time::sleep(Duration::from_millis(500)).await;

        let events = observer.drain_events();
        let has_large = events.iter().any(|e| e.event_type == "LargeFileDetected");
        assert!(
            has_large,
            "should detect large file, events: {:?}",
            events.iter().map(|e| &e.event_type).collect::<Vec<_>>()
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
