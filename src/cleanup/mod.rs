pub mod history;
pub mod policy;
pub mod preflight;
pub mod transaction;
pub mod undo;
pub mod verification;

#[cfg(test)]
pub mod executor_tests;

#[allow(unused_imports)]
pub use policy::{CleanupAction, CleanupPolicy, RiskLevel};
#[allow(unused_imports)]
pub use preflight::{build_candidates, CleanupCandidate};
#[allow(unused_imports)]
pub use transaction::{CleanupExecutor, CleanupPlan};
#[allow(unused_imports)]
pub use undo::{CleanupActionRecord, CleanupTransaction};
