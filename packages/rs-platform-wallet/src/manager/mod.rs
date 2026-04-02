#[cfg(feature = "manager")]
mod platform_wallet_manager;
#[cfg(feature = "manager")]
pub(crate) mod spv_event_forwarder;
#[cfg(feature = "manager")]
pub mod spv_runtime;
#[cfg(feature = "manager")]
pub(crate) mod spv_wallet_adapter;

#[cfg(feature = "manager")]
pub use platform_wallet_manager::PlatformWalletManager;
