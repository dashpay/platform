//! Platform wallet with identity management

pub mod error;
pub mod events;
#[cfg(feature = "manager")]
pub mod manager;
pub mod persistence;
#[cfg(feature = "manager")]
pub(crate) mod spv;
pub mod wallet;

pub use error::PlatformWalletError;
pub use events::PlatformWalletEvent;
pub use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
#[cfg(feature = "manager")]
pub use manager::PlatformWalletManager;
#[cfg(feature = "manager")]
pub use spv::SpvRuntime;
pub use wallet::core::WalletBalance;
pub use wallet::core::{AssetLockStatus, CoreAddressInfo, CoreWallet, TrackedAssetLock};
pub use wallet::dashpay::ContactRequest;
pub use wallet::dashpay::EstablishedContact;
pub use wallet::dashpay::{
    calculate_account_reference, derive_contact_payment_address, derive_contact_payment_addresses,
    derive_contact_xpub, ContactXpubData, DEFAULT_CONTACT_GAP_LIMIT,
};
pub use wallet::identity::managed_identity::BlockTime;
pub use wallet::identity::IdentityManager;
pub use wallet::identity::ManagedIdentity;
pub use wallet::identity::WatchedIdentity;
pub use wallet::identity::{
    DpnsNameInfo, IdentityFundingMethod, IdentityStatus, KeyStorage, PrivateKeyData,
    TopUpFundingMethod,
};
pub use wallet::ManagedIdentitySigner;
pub use wallet::PlatformWallet;
pub use wallet::TokenWallet;

#[cfg(feature = "manager")]
pub use key_wallet_manager;
