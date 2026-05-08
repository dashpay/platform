//! Platform wallet with identity management.
//!
//! `platform-wallet` ties together a `key-wallet` HD wallet (UTXO state,
//! key derivation), a Dash Platform identity manager, asset locks,
//! platform-address tracking, and an optional SPV runtime into a single
//! coherent abstraction. Most callers should drive it through
//! [`PlatformWalletManager`], which owns one or more
//! [`PlatformWallet`]s and the shared SPV runtime.
//!
//! # Module map
//!
//! - [`manager`] — top-level [`PlatformWalletManager`] (wallet
//!   lifecycle, SPV start/stop, accessors).
//! - [`wallet`] — the [`PlatformWallet`] aggregate plus the
//!   sub-wallets that make it up: `core`, `identity`, `platform_addresses`
//!   (reached via [`PlatformWallet::platform`]), `tokens`, `asset_lock`,
//!   and the optional `shielded` pool.
//! - [`changeset`] — delta types persisted on every mutation; the apply
//!   path replays them to rebuild in-memory state on startup.
//! - [`broadcaster`] — pluggable [`TransactionBroadcaster`](broadcaster::TransactionBroadcaster)
//!   (DAPI or SPV).
//! - [`spv`] — SPV runtime used by the SPV broadcaster and by sync.
//! - [`events`] — fan-out event manager and handlers.
//! - [`platform_address_sync`] — periodic platform-address balance sync
//!   coordinator.
//! - [`error`] — the unified [`PlatformWalletError`] type.
//!
//! # Quick start
//!
//! See `examples/basic_usage.rs` for an end-to-end walkthrough — create a
//! manager with `PlatformWalletManager::new`, build a wallet from a seed
//! with `create_wallet_from_seed_bytes`, then read balances or derive
//! receive addresses through the returned [`PlatformWallet`] handle.

// The crate's error enum wraps several large variants (SDK errors, DPP
// consensus errors, etc.). Shrinking it (e.g. boxing variants) would be a
// broader refactor; allow the lints for now.
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
// Test-code-heavy lints used intentionally across this crate's unit tests.
#![cfg_attr(test, allow(clippy::bool_assert_comparison))]
#![cfg_attr(test, allow(clippy::field_reassign_with_default))]
// Doc list formatting nits from our Markdown-style bullet lists in rustdoc.
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::doc_overindented_list_items)]

pub mod broadcaster;
pub mod changeset;
pub mod error;
pub mod events;
pub mod manager;
pub mod platform_address_sync;
pub mod spv;
pub mod wallet;

pub use error::PlatformWalletError;
pub use events::{PlatformEventHandler, PlatformEventManager};
pub use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
pub use manager::PlatformWalletManager;
pub use platform_address_sync::{
    PlatformAddressSyncManager, PlatformAddressSyncSummary, WalletSyncOutcome,
    DEFAULT_SYNC_INTERVAL_SECS,
};
pub use spv::SpvRuntime;
pub use wallet::asset_lock::manager::AssetLockManager;
pub use wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
pub use wallet::core::CoreWallet;
pub use wallet::core::WalletBalance;
// DashPay types + crypto helpers re-exported through the identity
// domain (they live under `identity::types::dashpay::*` and
// `identity::crypto::*` internally).
pub use wallet::identity::network::{
    derive_identity_auth_keypair, IDENTITY_GAP_LIMIT, MASTER_KEY_INDEX,
};
pub use wallet::identity::{
    calculate_account_reference, derive_auto_accept_private_key, derive_contact_payment_address,
    derive_contact_payment_addresses, derive_contact_xpub, BlockTime, ContactRequest,
    ContactXpubData, DashPayProfile, DpnsNameInfo, EstablishedContact, IdentityFunding,
    IdentityFundingMethod, IdentityLocation, IdentityManager, IdentityStatus, KeyStorage,
    ManagedIdentity, PrivateKeyData, ProfileUpdate, RegistrationIndex, TopUpFundingMethod,
    DEFAULT_CONTACT_GAP_LIMIT,
};
pub use wallet::platform_wallet::PlatformWalletInfo;
pub use wallet::ManagedIdentitySigner;
pub use wallet::PlatformAddressTag;
pub use wallet::PlatformWallet;
pub use wallet::TokenWallet;

// Re-export changeset types for caller-level staging.
pub use changeset::Merge;
pub use changeset::{
    AssetLockChangeSet, AssetLockEntry, ClientStartState, ClientWalletStartState, ContactChangeSet,
    ContactRequestEntry, IdentityChangeSet, IdentityEntry, IdentityKeyEntry, IdentityKeysChangeSet,
    IdentityManagerStartState, PlatformAddressBalanceEntry, PlatformAddressChangeSet,
    PlatformAddressSyncStartState, PlatformWalletChangeSet, TokenBalanceChangeSet,
};

pub use key_wallet_manager;

// Re-export the per-wallet persistence handle so callers outside
// the crate can pass it to `ManagedIdentity` mutation methods
// (`set_dashpay_profile`, `record_dashpay_payment`, `add_identity`, …).
pub use wallet::persister::WalletPersister;
