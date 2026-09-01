//! Shared internal helpers (safe casts, file permissions, etc.).

pub mod permissions;
pub mod safe_cast;
pub(super) mod wallet;

/// Rebuild a `ManagedWalletInfo` from persisted core state — the apply half
/// of a rehydration, paired with `sqlite::schema::core_state::load_state`.
pub use wallet::apply_persisted_core_state;
