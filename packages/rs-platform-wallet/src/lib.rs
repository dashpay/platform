//! Platform wallet with identity management

pub mod block_time;
pub mod error;
pub mod events;
pub mod manager;
pub mod wallet;

pub use block_time::BlockTime;
pub use error::PlatformWalletError;
pub use events::PlatformWalletEvent;
pub use manager::PlatformWalletManager;
pub use wallet::core::CoreWallet;
pub use wallet::dashpay::ContactRequest;
pub use wallet::dashpay::EstablishedContact;
pub use wallet::identity::IdentityManager;
pub use wallet::identity::ManagedIdentity;
pub use wallet::PlatformWallet;

#[cfg(feature = "manager")]
pub use key_wallet_manager;
