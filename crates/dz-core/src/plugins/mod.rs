//! DZSDK plugin system: manifests, permissions, sandboxed runtime host,
//! declarative view model and the on-disk registry.

pub mod host;
pub mod manifest;
pub mod permissions;
pub mod registry;
pub mod view;

/// The DZSDK API version implemented by this build.
pub const API_VERSION: &str = "1";
