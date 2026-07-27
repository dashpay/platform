//! Asset lock lifecycle management.
//!
//! Tracks asset lock transactions from build through finality (IS/CL) and
//! Platform consumption. Shared across sub-wallets via `Arc<AssetLockManager>`.

mod build;
pub mod lock_notify_handler;
pub mod manager;
pub mod orchestration;
mod sync;
pub mod tracked;

pub use lock_notify_handler::LockNotifyHandler;
pub use orchestration::AssetLockFunding;
