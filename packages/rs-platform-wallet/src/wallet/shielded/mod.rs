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

pub mod wallet_drain;
pub use wallet_drain::{WalletDrainPreview, WalletDrainPreviewStatus};
pub mod activity;
pub mod activity_recorder;
pub mod balance;
pub(crate) mod balance_emitter;
#[cfg(test)]
mod balance_emitter_tests;
#[cfg(test)]
mod balance_tests;
#[cfg(test)]
mod cancelled_scan_tests;
pub(crate) mod contact_address;
#[cfg(test)]
mod contact_address_tests;
pub mod coordinator;
pub mod file_store;
#[cfg(test)]
mod fresh_address_scan_tests;
pub mod fund_from_asset_lock;
pub mod keys;
pub mod note_selection;
pub mod operations;
pub mod prover;
pub mod seed_pool;
pub mod store;
pub mod sync;
pub mod tip_payment;
#[cfg(test)]
mod tip_payment_tests;
mod tip_recipients;
pub mod tips;
pub mod token_pool;
pub mod wallet_drain_recovery;
pub use tip_payment::{
    parse_shielded_tip_amount, ShieldedIdentityBalance, ShieldedTipAdmission, ShieldedTipRecipient,
    ShieldedTipRecipientChange, ShieldedTipReview, ShieldedTipSource,
    SHIELDED_IDENTITY_BALANCE_WAIT,
};
pub use tips::{
    is_shielded_identity_account, shielded_identity_account_index, SHIELDED_IDENTITY_ACCOUNT_BASE,
};
#[cfg(test)]
mod spend_admission_tests;
#[cfg(test)]
pub(crate) mod viewing_key_bind_tests;
#[cfg(test)]
mod wallet_drain_tests;

pub use activity::{
    compute_activity_id, derive_activity_from_scan_data, sort_activity_for_display,
    ScanDeriveInput, ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus,
    ShieldedDirection,
};
pub use balance::{
    ShieldedBalanceSource, ShieldedBalanceUpdate, ShieldedLocalAccountBalance,
    ShieldedLocalBalanceSnapshot, ShieldedLocalBalanceState, ShieldedOrdinaryBalance,
};
pub(crate) use contact_address::ContactShieldedAddresses;
pub use coordinator::NetworkShieldedCoordinator;
pub use file_store::{FileBackedShieldedStore, FileShieldedStoreError};
pub use keys::{AccountViewingKeys, OrchardKeySet};
pub use note_selection::TokenPoolSpendKind;
pub use operations::{
    ShieldedTokenPurchaseAvailability, ShieldedTokenPurchaseOutcome, ShieldedTokenSpendOutcome,
    ShieldedTokenSpendPreview,
};
pub use prover::CachedOrchardProver;
pub use seed_pool::{SeedPoolOutcome, SeedPoolProgress, DEFAULT_SEED_POOL_TARGET_NOTES};
pub use store::{
    IdentityDebitRecoveryRecord, IdentityDebitRecoveryStatus, InMemoryShieldedStore,
    PrivateContactPayment, PrivatePaymentEntry, PrivatePaymentRecord, ShieldedNote,
    ShieldedOutgoingNote, ShieldedStore, SubwalletId,
};
pub use sync::{ShieldedSyncSummary, SyncNotesResult};
pub use token_pool::{ShieldedTokenPoolBalance, ShieldedTokenPoolWatch};

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
