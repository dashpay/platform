//! Platform wallet with identity management

pub mod changeset;
pub mod error;
pub mod events;
pub mod manager;
pub(crate) mod spv;
pub mod wallet;

pub use error::PlatformWalletError;
pub use events::PlatformWalletEvent;
pub use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
pub use manager::PlatformWalletManager;
pub use spv::SpvRuntime;
pub use wallet::asset_lock::manager::AssetLockManager;
pub use wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
pub use wallet::core::WalletBalance;
pub use wallet::core::{CoreAddressInfo, CoreWallet};
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
pub use wallet::ManagedIdentitySigner;
pub use wallet::PlatformWallet;
pub use wallet::TokenWallet;

// Re-export changeset types for caller-level staging.
pub use changeset::Merge;
pub use changeset::{
    AccountChangeSet, AssetLockChangeSet, AssetLockEntry, ChainChangeSet, ContactChangeSet,
    ContactRequestEntry, IdentityChangeSet, IdentityEntry, PlatformAddressChangeSet,
    PlatformAddressEntry, PlatformWalletChangeSet, TransactionChangeSet, TransactionEntry,
    UtxoChangeSet,
};

pub use key_wallet_manager;
