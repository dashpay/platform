//! [`PlatformWallet`] aggregate plus its sub-wallets (core, identity,
//! platform addresses, tokens, asset locks, optional shielded pool).

pub mod apply;
pub mod asset_lock;
pub mod core;
pub mod identity;
pub mod persister;
pub mod platform_addresses;
pub mod platform_wallet;
mod platform_wallet_traits;
#[cfg(feature = "shielded")]
pub mod shielded;
pub mod signer;
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
pub use signer::{IdentitySigner, ManagedIdentitySigner};
pub use tokens::TokenWallet;
