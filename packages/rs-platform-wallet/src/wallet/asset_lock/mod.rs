//! Asset lock lifecycle management.
//!
//! Tracks asset lock transactions from build through finality (IS/CL) and
//! Platform consumption. Shared across sub-wallets via `Arc<AssetLockManager>`.

mod build;
mod operation;
pub use operation::{
    AssetLockOperationContext, AssetLockOperationRegistration, AssetLockOperationSnapshot,
    AssetLockOperationState, MAX_DRAIN_ATTEMPTS, MAX_DRAIN_ATTEMPT_BYTES,
};
pub mod lock_notify_handler;
pub mod manager;
pub mod orchestration;
pub(crate) mod sync;
pub mod tracked;

pub use lock_notify_handler::LockNotifyHandler;
pub use orchestration::AssetLockFunding;
