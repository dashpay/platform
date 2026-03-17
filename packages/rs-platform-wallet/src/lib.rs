//! Platform wallet with identity management

pub mod block_time;
pub mod contact_request;
pub mod crypto;
pub mod error;
pub mod established_contact;
pub mod events;
pub mod identity_manager;
pub mod managed_identity;
pub mod manager;
pub mod wallet;

pub use block_time::BlockTime;
pub use contact_request::ContactRequest;
pub use error::PlatformWalletError;
pub use established_contact::EstablishedContact;
pub use events::PlatformWalletEvent;
pub use identity_manager::IdentityManager;
pub use managed_identity::ManagedIdentity;
pub use manager::PlatformWalletManager;
pub use wallet::PlatformWallet;
pub use wallet::core_wallet::CoreWallet;

#[cfg(feature = "manager")]
pub use key_wallet_manager;
