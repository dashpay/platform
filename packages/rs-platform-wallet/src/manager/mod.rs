#[cfg(feature = "manager")]
pub(crate) mod spv_wallet_adapter;
mod platform_wallet_manager;

pub use platform_wallet_manager::PlatformWalletManager;
