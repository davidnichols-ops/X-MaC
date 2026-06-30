//! The `envmap` engine — environment mapping (MIF Environment Mapper port).
//!
//! Submodules:
//! - [`redaction`] — privacy-first redactor (MIF `DataSanitizer` port).
//! - [`discovery`] — system + language package discovery (MIF `DiscoveryEngine` port).
//! - [`apps`] — installed application enumeration via `Info.plist`.
//! - [`engine`] — the `Engine` trait implementation that ties it together.

pub mod apps;
pub mod discovery;
pub mod engine;
pub mod redaction;

pub use engine::EnvmapEngine;
