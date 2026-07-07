pub mod apply;
pub mod asset_lock;
pub mod core;
pub mod identity;
pub mod persister;
pub mod platform_addresses;
pub mod platform_wallet;
mod platform_wallet_traits;
pub(crate) mod reservations;
#[cfg(feature = "shielded")]
pub mod shielded;
pub mod tokens;

pub use self::core::CoreWallet;
pub use apply::ApplyError;
pub use identity::IdentityWallet;
pub use platform_addresses::{
    PerAccountPlatformAddressState, PerWalletPlatformAddressState, PlatformAddressTag,
    PlatformAddressWallet,
};
pub use platform_wallet::{
    PlatformWallet, PlatformWalletInfo, WalletId, WalletStateReadGuard, WalletStateWriteGuard,
};
