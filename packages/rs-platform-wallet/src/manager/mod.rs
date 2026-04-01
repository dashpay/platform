mod platform_wallet_manager;
#[cfg(feature = "manager")]
pub(crate) mod spv_event_forwarder;
#[cfg(feature = "manager")]
pub(crate) mod spv_wallet_adapter;

pub use platform_wallet_manager::PlatformWalletManager;
