//! Shared internal helpers (safe casts, file permissions, etc.).

pub mod permissions;
pub mod safe_cast;
pub(super) mod wallet;

/// Integration-test access to the crate-internal core-state reconstruction
/// helper. Gated so the normal build keeps `apply_persisted_core_state`
/// private to the crate.
#[cfg(feature = "rehydration-apply")]
pub use wallet::apply_persisted_core_state;
