//! Asset lock lifecycle management.
//!
//! Tracks asset lock transactions from build through finality (IS/CL) and
//! Platform consumption. Shared across sub-wallets via `Arc<AssetLockManager>`.

use dashcore::Transaction;

use crate::error::PlatformWalletError;

mod build;
mod operation;
pub use operation::{
    AssetLockOperationContext, AssetLockOperationRegistration, AssetLockOperationSnapshot,
    AssetLockOperationState,
};
pub mod lock_notify_handler;
pub mod manager;
pub mod orchestration;
pub mod restored_record;
pub(crate) mod sync;
pub mod tracked;

pub use build::AssetLockBuiltObserver;
pub use lock_notify_handler::LockNotifyHandler;
pub use orchestration::AssetLockFunding;

/// The asset-lock input cap this protocol version enforces
/// (`max_asset_lock_transaction_inputs`, 100 since v3). Dash Core's relay
/// policy refuses the same bound (`MAX_STANDARD_ASSET_LOCK_INPUTS`,
/// `assetlocktx-too-many-inputs`), and peers drop such a lock silently, so
/// every broadcasting build refuses it before anything reaches the wire. The
/// build-only `build_asset_lock_transaction*` does not check it: refusing it
/// at construction belongs in key-wallet's builder (dashpay/rust-dashcore#1190).
/// Tighter than key-wallet's standard-transaction input limit.
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

/// The refusal an asset-lock transaction over the input cap draws, or
/// `None` when it fits.
///
/// The one place the cap is judged on a built transaction. A funded build
/// asks it before the lock is tracked or sent (`build.rs`), and a resume of
/// a tracked `Built` / `Broadcast` row asks it before any re-send
/// (`sync/recovery.rs`): a row built by a version before the pre-broadcast
/// check, or by the build-only API, is otherwise re-sent on every resume,
/// dropped silently by every peer, and never settles.
pub(crate) fn input_cap_violation(
    transaction: &Transaction,
    platform_version: &dpp::version::PlatformVersion,
) -> Option<PlatformWalletError> {
    let max = max_asset_lock_inputs(platform_version);
    let count = transaction.input.len();
    (count > max).then_some(PlatformWalletError::AssetLockTooManyInputs { count, max })
}
