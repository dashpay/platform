//! Asset lock lifecycle management.
//!
//! Tracks asset lock transactions from build through finality (IS/CL) and
//! Platform consumption. Shared across sub-wallets via `Arc<AssetLockManager>`.

mod build;
mod operation;
pub use operation::{
    AssetLockOperationContext, AssetLockOperationRegistration, AssetLockOperationSnapshot,
    AssetLockOperationState,
};
pub mod lock_notify_handler;
pub mod manager;
pub mod orchestration;
pub(crate) mod sync;
pub mod tracked;

pub use lock_notify_handler::LockNotifyHandler;
pub use orchestration::AssetLockFunding;

/// The asset-lock input cap this protocol version holds an instant asset
/// lock proof to (`max_asset_lock_transaction_inputs`, 100 from protocol
/// version 11; from protocol version 15 a chain asset lock proof is not
/// held to it). Dash Core's relay
/// policy refuses the same bound (`MAX_STANDARD_ASSET_LOCK_INPUTS`,
/// `assetlocktx-too-many-inputs`), and peers drop such a lock silently, so a
/// wallet must refuse it before anything reaches the wire. Tighter than
/// key-wallet's standard-transaction input limit.
pub(crate) fn max_asset_lock_inputs(version: &dpp::version::PlatformVersion) -> usize {
    usize::from(
        version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .max_asset_lock_transaction_inputs,
    )
}
