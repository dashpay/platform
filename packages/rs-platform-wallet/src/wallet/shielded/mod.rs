//! Feature-gated shielded (Orchard / Halo 2) wallet support.
//!
//! This module provides ZK-private transactions on Dash Platform
//! using the Orchard circuit (Halo 2 proving system). It is
//! gated behind the `shielded` Cargo feature because it pulls in
//! heavy cryptographic dependencies.
//!
//! # Architecture
//!
//! - [`OrchardKeySet`] — ZIP-32 key derivation from a wallet
//!   seed (FVK / IVK / OVK / `SpendAuthorizingKey` / default
//!   payment address).
//! - [`AccountViewingKeys`] — viewing-grade subset of
//!   `OrchardKeySet` (no ASK); the only key material that
//!   crosses to coordinator scope.
//! - [`NetworkShieldedCoordinator`] — network-scoped owner of
//!   the shared commitment-tree store, the per-`SubwalletId`
//!   account registry, the caught-up cooldown stamp, and the
//!   sync entry point. One coordinator per
//!   `PlatformWalletManager`.
//! - [`ShieldedStore`] / [`InMemoryShieldedStore`] /
//!   [`FileBackedShieldedStore`] — storage abstraction; the
//!   shared commitment tree lives here. Per-subwallet notes are
//!   scoped by [`SubwalletId`] inside the store.
//! - [`CachedOrchardProver`] — lazy-init proving key cache.
//! - Sync / spend operations live as free functions in the
//!   [`sync`] and [`operations`] submodules and take
//!   `(sdk, store, persister, wallet_id, keys, …)` explicitly.
//!
//! Per-wallet shielded state on `PlatformWallet` is just the
//! `BTreeMap<u32, OrchardKeySet>` (with the spend authority);
//! `PlatformWallet` doesn't need a wrapper struct around it.

pub mod activity;
pub mod activity_recorder;
pub mod coordinator;
pub mod file_store;
pub mod fund_from_asset_lock;
pub mod keys;
pub mod note_selection;
pub mod operations;
pub mod prover;
pub mod seed_pool;
pub mod store;
pub mod sync;

pub use activity::{
    compute_activity_id, derive_activity_from_scan_data, sort_activity_for_display,
    ScanDeriveInput, ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus,
    ShieldedDirection,
};
pub use coordinator::NetworkShieldedCoordinator;
pub use file_store::{FileBackedShieldedStore, FileShieldedStoreError};
pub use keys::{AccountViewingKeys, OrchardKeySet};
pub use prover::CachedOrchardProver;
pub use seed_pool::{SeedPoolOutcome, SeedPoolProgress, DEFAULT_SEED_POOL_TARGET_NOTES};
pub use store::{
    InMemoryShieldedStore, ShieldedNote, ShieldedOutgoingNote, ShieldedStore, SubwalletId,
};
pub use sync::{ShieldedSyncSummary, SyncNotesResult};

/// How long after a no-op sync the background loop should skip
/// further passes. Tuned against the wallet's typical 60s sync
/// cadence — halves wire calls in steady-state while keeping
/// "new notes" discovery latency bounded at one cooldown window
/// plus the next loop tick. Manual `force=true` syncs bypass.
///
/// Held in the coordinator (`NetworkShieldedCoordinator::sync`
/// is the only caller); the per-`PlatformWallet` cooldown stamp
/// was removed in Phase 4a.
pub(super) const CAUGHT_UP_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(30);
