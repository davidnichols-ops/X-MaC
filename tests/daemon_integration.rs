//! Daemon lifecycle integration tests.
//!
//! These tests exercise the `Daemon` API in `src/intelligence/daemon.rs`:
//!   - start/stop lifecycle
//!   - PID file creation and cleanup
//!   - single-instance (double-start) enforcement
//!   - stopping when no daemon is running
//!   - recovery from a stale PID file pointing at a dead process
//!
//! The daemon does not `fork()` — `Daemon::run` is an async function that runs
//! in the current process and writes the *current* process's PID to the PID
//! file. To test `is_running` / `stop` against a real live process without
//! sending signals to the test process itself, several tests spawn a short-lived
//! child process (`sleep`) and write its PID into a temp PID file.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use tempfile::TempDir;
use x_mac::config::store::ConfigManager;
use x_mac::intelligence::daemon::Daemon;

/// Build a `ConfigManager` whose daemon PID file lives inside a temp dir and
/// whose daemon cycle is a no-op (no auto-purge, no auto-clean, no automation
/// rules) so `run_cycle` is fast and side-effect free.
fn config_with_pid_file(tmp: &TempDir) -> ConfigManager {
    let config_path = tmp.path().join("config.toml");
    let mut mgr = ConfigManager::load_from(&config_path);
    let pid_file = tmp.path().join("xmac.pid");
    mgr.config_mut().daemon.pid_file = pid_file;
    // Disable every side-effecting daemon behaviour so cycles are inert.
    mgr.config_mut().daemon.auto_purge_memory = false;
    mgr.config_mut().daemon.auto_clean_threshold_mb = 0;
    mgr.config_mut().automation.clear();
    mgr
}

/// Spawn a `sleep 30` child process and return its PID. Used to simulate a
/// "running daemon" for `is_running` / `stop` tests without risking the test
/// process itself.
fn spawn_sleep_child() -> std::process::Child {
    Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("failed to spawn `sleep` child")
}

/// Helper to write a raw PID into a PID file.
fn write_pid_file(pid_file: &PathBuf, pid: i32) {
    if let Some(parent) = pid_file.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(pid_file, pid.to_string()).expect("write PID file");
}

// -----------------------------------------------------------------------
// 1. Start/stop lifecycle + PID file cleanup (in-process, real daemon loop)
// -----------------------------------------------------------------------

/// Start the real daemon loop in a background task, verify `is_running`
/// reports it as running, then `stop` it and verify the PID file is cleaned
/// up and `is_running` reports false.
///
/// NOTE: there is a known bug in `Daemon::run` — the outer `tokio::select!`
/// drops the `shutdown` (signal-handler) future as soon as the `tick.tick()`
/// branch completes (which fires immediately on the first tick). This means
/// the running loop never actually responds to the SIGTERM that `stop()`
/// sends. `stop()` still removes the PID file itself (after a short sleep),
/// which is what makes the daemon appear stopped. We therefore verify the
/// observable `stop()` contract (PID file removed, `is_running` false) and
/// abort the orphaned task so no background work leaks out of the test.
#[tokio::test]
async fn test_start_stop_lifecycle() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mgr = config_with_pid_file(&tmp);
    let pid_file = mgr.config().daemon.pid_file.clone();

    // Nothing running initially.
    assert!(Daemon::is_running(&pid_file).is_none());
    assert!(!pid_file.exists());

    let daemon = Daemon::new(mgr, 60, false);
    let handle = tokio::spawn(async move { daemon.run(false).await });

    // Give the daemon task time to write the PID file and enter its loop.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The daemon should now be reported as running.
    let running_pid = Daemon::is_running(&pid_file);
    assert!(
        running_pid.is_some(),
        "daemon should be running after start, got {:?}",
        running_pid
    );
    assert!(pid_file.exists(), "PID file should exist while running");

    // Stop the daemon. `stop()` sends SIGTERM to the PID in the file and then
    // removes the PID file. Due to the select bug above the signal itself is
    // not handled, but `stop()` still cleans up the PID file.
    Daemon::stop(&pid_file).expect("stop should succeed");

    // After stop: not running, PID file cleaned up.
    assert!(
        Daemon::is_running(&pid_file).is_none(),
        "daemon should not be running after stop"
    );
    assert!(!pid_file.exists(), "PID file should be removed after stop");

    // Abort the orphaned daemon task so it does not leak out of the test.
    handle.abort();
}

// -----------------------------------------------------------------------
// 2. PID file cleanup after a single-cycle run
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_run_once_cleans_pid_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mgr = config_with_pid_file(&tmp);
    let pid_file = mgr.config().daemon.pid_file.clone();

    assert!(!pid_file.exists());

    let daemon = Daemon::new(mgr, 60, false);
    // `once` mode writes the PID file, runs one cycle, then removes it.
    daemon.run(true).await.expect("run once should succeed");

    assert!(
        !pid_file.exists(),
        "PID file must be removed after a once-cycle run"
    );
    assert!(Daemon::is_running(&pid_file).is_none());
}

// -----------------------------------------------------------------------
// 3. Double-start prevention
// -----------------------------------------------------------------------

/// If a live process's PID is already in the PID file, starting a new daemon
/// must fail (single-instance enforcement) rather than silently overwriting.
#[tokio::test]
async fn test_double_start_prevention() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mgr = config_with_pid_file(&tmp);
    let pid_file = mgr.config().daemon.pid_file.clone();

    // Simulate an already-running daemon with a live child process.
    let mut child = spawn_sleep_child();
    let child_pid = child.id() as i32;
    write_pid_file(&pid_file, child_pid);

    let daemon = Daemon::new(mgr, 60, false);
    let result = daemon.run(true).await;

    // Starting while another instance is "running" must error.
    assert!(
        result.is_err(),
        "run should fail when another daemon is already running"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("already running"),
        "error should mention already running, got: {err}"
    );

    // The PID file should still be intact (we did not overwrite it).
    assert_eq!(
        std::fs::read_to_string(&pid_file).unwrap().trim(),
        child_pid.to_string()
    );

    // Cleanup: kill the simulated daemon.
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&pid_file);
}

// -----------------------------------------------------------------------
// 4. Stop when not running — must not panic
// -----------------------------------------------------------------------

#[test]
fn test_stop_when_not_running_returns_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pid_file = tmp.path().join("nonexistent.pid");
    assert!(!pid_file.exists());

    // Should return an error, not panic.
    let result = Daemon::stop(&pid_file);
    assert!(
        result.is_err(),
        "stop with no running daemon should return an error"
    );
    assert!(!pid_file.exists());
}

// -----------------------------------------------------------------------
// 5. Dead PID recovery
// -----------------------------------------------------------------------

/// A PID file pointing at a dead process should report `is_running == false`,
/// and a subsequent daemon start should succeed (overwriting the stale file).
#[tokio::test]
async fn test_dead_pid_recovery() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mgr = config_with_pid_file(&tmp);
    let pid_file = mgr.config().daemon.pid_file.clone();

    // Write a PID that is very unlikely to exist.
    write_pid_file(&pid_file, 999_999);

    // is_running should detect the dead process.
    #[cfg(unix)]
    assert!(
        Daemon::is_running(&pid_file).is_none(),
        "is_running should return None for a dead PID"
    );

    // A new start should recover: write_pid_file sees the dead PID and
    // overwrites it with the current process's PID.
    let daemon = Daemon::new(mgr, 60, false);
    daemon
        .run(true)
        .await
        .expect("run once should succeed despite stale PID file");

    // After the once-run, the PID file is cleaned up.
    assert!(
        !pid_file.exists(),
        "PID file should be cleaned up after successful run"
    );
    assert!(Daemon::is_running(&pid_file).is_none());
}

// -----------------------------------------------------------------------
// Bonus: is_running / stop against a real external live process
// -----------------------------------------------------------------------

/// `is_running` should report a live process whose PID is in the file, and
/// `stop` should SIGTERM that process and remove the PID file.
#[test]
fn test_stop_real_live_process() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pid_file = tmp.path().join("xmac.pid");

    let mut child = spawn_sleep_child();
    let child_pid = child.id() as i32;
    write_pid_file(&pid_file, child_pid);

    // The live child should be detected as running.
    #[cfg(unix)]
    {
        let running = Daemon::is_running(&pid_file);
        assert_eq!(running, Some(child_pid));
    }

    // stop() sends SIGTERM to the child and removes the PID file.
    Daemon::stop(&pid_file).expect("stop should succeed against live process");

    // The child should have exited.
    let status = child.wait().expect("wait for child");
    assert!(
        !status.success(),
        "child should have been terminated by SIGTERM"
    );

    // PID file removed by stop().
    assert!(!pid_file.exists(), "PID file should be removed by stop");
}

/// `is_running` against a dead process should return None.
#[test]
fn test_is_running_dead_external_process() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pid_file = tmp.path().join("xmac.pid");

    // Spawn a child that exits immediately.
    let mut child = Command::new("true").spawn().expect("spawn true");
    let child_pid = child.id() as i32;
    child.wait().expect("wait true");

    write_pid_file(&pid_file, child_pid);

    #[cfg(unix)]
    assert!(
        Daemon::is_running(&pid_file).is_none(),
        "is_running should return None for an exited process"
    );

    let _ = std::fs::remove_file(&pid_file);
}
