// Observers — continuous system observation feeding the event store
//
// Each observer monitors one dimension of the Mac and emits events into
// the EventStore. The observers are the "sensors" that make the Digital
// Twin alive.
//
// Phase 1b observers:
//   process_observer     — polls ps every 5s, detects launches/terminations/anomalies
//   filesystem_observer  — uses FSEvents (via notify crate) for real-time file changes

#![allow(dead_code, unused_imports)]

pub mod filesystem_observer;
pub mod process_observer;
pub mod runner;

pub use filesystem_observer::FilesystemObserver;
pub use process_observer::ProcessObserver;
pub use runner::ObserverRunner;
