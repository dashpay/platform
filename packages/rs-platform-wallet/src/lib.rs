//! Platform wallet with identity management

pub mod broadcaster;
pub mod changeset;
pub mod error;
pub mod events;
pub mod manager;
pub mod spv;
pub mod wallet;

pub use error::PlatformWalletError;
pub use events::{PlatformEventHandler, PlatformEventManager};
pub use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
pub use manager::PlatformWalletManager;
pub use spv::SpvRuntime;
pub use wallet::asset_lock::manager::AssetLockManager;
pub use wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
pub use wallet::core::CoreWallet;
pub use wallet::core::WalletBalance;
pub use wallet::dashpay::ContactRequest;
pub use wallet::dashpay::EstablishedContact;
pub use wallet::dashpay::{
    calculate_account_reference, derive_auto_accept_private_key, derive_contact_payment_address,
    derive_contact_payment_addresses, derive_contact_xpub, ContactXpubData,
    DEFAULT_CONTACT_GAP_LIMIT,
};
pub use wallet::identity::managed_identity::BlockTime;
pub use wallet::identity::IdentityManager;
pub use wallet::identity::ManagedIdentity;
pub use wallet::identity::WatchedIdentity;
pub use wallet::identity::{
    DpnsNameInfo, IdentityFunding, IdentityFundingMethod, IdentityStatus, KeyStorage,
    PrivateKeyData, TopUpFundingMethod,
};
pub use wallet::platform_wallet::PlatformWalletInfo;
pub use wallet::ManagedIdentitySigner;
pub use wallet::PlatformWallet;
pub use wallet::TokenWallet;

// Re-export changeset types for caller-level staging.
pub use changeset::Merge;
pub use changeset::{
    AssetLockChangeSet, AssetLockEntry, ContactChangeSet, ContactRequestEntry, IdentityChangeSet,
    IdentityEntry, PlatformAddressChangeSet, PlatformWalletChangeSet, TokenBalanceChangeSet,
};

pub use key_wallet_manager;

// Re-export the per-wallet persistence handle so callers outside
// the crate can pass it to `ManagedIdentity` mutation methods
// (`set_dashpay_profile`, `record_dashpay_payment`, `add_identity`, …).
pub use wallet::persister::WalletPersister;
