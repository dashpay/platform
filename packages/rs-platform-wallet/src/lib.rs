//! Platform wallet with identity management

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
pub use wallet::identity::{
    calculate_account_reference, derive_auto_accept_private_key, derive_contact_payment_address,
    derive_contact_payment_addresses, derive_contact_xpub, BlockTime, ContactRequest,
    ContactXpubData, DpnsNameInfo, EstablishedContact, IdentityFunding, IdentityFundingMethod,
    IdentityManager, IdentityStatus, KeyStorage, ManagedIdentity, PrivateKeyData,
    TopUpFundingMethod, WatchedIdentity, DEFAULT_CONTACT_GAP_LIMIT,
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
    ContactRequestEntry, IdentityChangeSet, IdentityEntry, IdentityManagerStartState,
    PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformAddressSyncStartState,
    PlatformWalletChangeSet, TokenBalanceChangeSet,
};

pub use key_wallet_manager;

// Re-export the per-wallet persistence handle so callers outside
// the crate can pass it to `ManagedIdentity` mutation methods
// (`set_dashpay_profile`, `record_dashpay_payment`, `add_identity`, …).
pub use wallet::persister::WalletPersister;
