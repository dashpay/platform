#[cfg(feature = "manager")]
mod platform_wallet_manager;

#[cfg(feature = "manager")]
pub use platform_wallet_manager::PlatformWalletManager;
