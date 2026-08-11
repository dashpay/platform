//! Platform wallet with identity management

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

pub mod address_paths;
pub mod broadcaster;
pub mod changeset;
pub mod error;
pub mod events;
pub mod manager;
pub mod spv;
#[cfg(any(test, feature = "test-utils"))]
pub mod test_support;
mod util;
pub mod wallet;

pub use error::PlatformWalletError;
pub use events::{PlatformEventHandler, PlatformEventManager};
pub use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
// Surface the upstream `DerivedAddress` event payload through this
// crate so downstream FFI consumers (rs-platform-wallet-ffi) can
// project `CoreChangeSet.addresses_derived` without taking an extra
// direct dependency on `key-wallet-manager`.
pub use key_wallet_manager::DerivedAddress;
// Re-export the path-rendering helpers so FFI shims and other
// consumers can render `DerivedAddress.derivation_path` without
// reimplementing the layout rules.
pub use address_paths::{
    derivation_path_for_derived_address, derivation_path_string_for_derived_address,
};
pub use manager::dashpay_sync::{
    DashPaySyncManager, DashPaySyncSummary, WalletDashPaySyncOutcome,
    DEFAULT_SYNC_INTERVAL_SECS as DASHPAY_SYNC_DEFAULT_INTERVAL_SECS,
};
pub use manager::identity_sync::{
    IdentitySyncManager, IdentityTokenSyncInfo, IdentityTokenSyncState,
    DEFAULT_SYNC_INTERVAL_SECS as IDENTITY_SYNC_DEFAULT_INTERVAL_SECS,
    MAX_TOKENS_PER_BALANCE_BATCH as IDENTITY_SYNC_MAX_TOKENS_PER_BATCH,
};
pub use manager::platform_address_sync::{
    PlatformAddressSyncManager, PlatformAddressSyncSummary, WalletSyncOutcome,
    DEFAULT_SYNC_INTERVAL_SECS,
};
pub use manager::PlatformWalletManager;
pub use spv::SpvRuntime;
pub use wallet::asset_lock::manager::AssetLockManager;
pub use wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
pub use wallet::asset_lock::AssetLockFunding;
pub use wallet::core::WalletBalance;
pub use wallet::core::{CoreWallet, SignedCoreTransaction, SEND_FUNDING_SOURCES};
pub use wallet::signed_payment_registry::{
    RegisterWrongGeneration, ReservationToken, SignedPaymentError, SignedPaymentRegistry,
};
// DashPay types + crypto helpers re-exported through the identity
// domain (they live under `identity::types::dashpay::*` and
// `identity::crypto::*` internally).
pub use wallet::core_address_key::CoreAddressPrivateKey;
pub use wallet::identity::network::{
    derive_identity_auth_keypair, AutoAcceptProofSource, ContactCryptoProvider, ContactInfoOpened,
    ContactInfoPublishOutcome, ContactInfoSealed, SeedBindingVerification, IDENTITY_GAP_LIMIT,
    MASTER_KEY_INDEX,
};
pub use wallet::identity::{
    calculate_account_reference, derive_auto_accept_private_key, derive_contact_payment_address,
    derive_contact_payment_addresses, derive_contact_xpub, pubkey_binds_expected_key_data,
    unmask_account_reference, BlockTime, ContactProfileEntry, ContactRequest, ContactXpubData,
    DashPayProfile, DashPayState, DpnsNameInfo, EstablishedContact, IdentityLocation,
    IdentityManager, IdentityStatus, KeyStorage, ManagedIdentity, PrivateKeyData, ProfileUpdate,
    RegistrationIndex, DEFAULT_CONTACT_GAP_LIMIT,
};
pub use wallet::platform_wallet::PlatformWalletInfo;
#[cfg(feature = "shielded")]
pub use wallet::platform_wallet::ShieldedShieldPreflight;
pub use wallet::provider_key_at_index::{ProviderDerivedKey, ProviderKeyKind};
#[cfg(feature = "shielded")]
pub use wallet::shielded::operations::shield_fee_reserve_credits;
pub use wallet::PlatformAddressTag;
pub use wallet::PlatformWallet;

// Re-export changeset types for caller-level staging.
pub use changeset::Merge;
pub use changeset::{
    AssetLockChangeSet, AssetLockEntry, ClientStartState, ClientWalletStartState, ContactChangeSet,
    ContactRequestEntry, IdentityChangeSet, IdentityEntry, IdentityKeyEntry, IdentityKeysChangeSet,
    IdentityManagerStartState, PlatformAddressBalanceEntry, PlatformAddressChangeSet,
    PlatformAddressSyncStartState, PlatformWalletChangeSet, TokenBalanceChangeSet,
};
pub use changeset::{PersistenceCapabilities, PERSISTENCE_CAPABILITIES_VERSION};

pub use key_wallet_manager;

// Re-export the per-wallet persistence handle so callers outside
// the crate can pass it to `ManagedIdentity` mutation methods
// (`set_dashpay_profile`, `record_dashpay_payment`, `add_identity`, …).
pub use wallet::persister::WalletPersister;
