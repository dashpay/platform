pub mod asset_lock;
pub mod core;
pub mod dashpay;
pub mod identity;
pub(crate) mod persister;
pub mod platform_addresses;
pub mod platform_wallet;
#[cfg(feature = "shielded")]
pub mod shielded;
pub mod signer;
pub mod tokens;

pub use self::core::CoreWallet;
pub use dashpay::DashPayWallet;
pub use identity::IdentityWallet;
pub use platform_addresses::PlatformAddressWallet;
pub use platform_wallet::{PlatformWallet, WalletId};
pub use signer::{IdentitySigner, ManagedIdentitySigner};
pub use tokens::TokenWallet;
