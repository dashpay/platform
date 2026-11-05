//! Asset lock lifecycle management.
//!
//! Tracks asset lock transactions from build through finality (IS/CL) and
//! Platform consumption. Shared across sub-wallets via `Arc<AssetLockManager>`.

use dashcore::Transaction;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::asset_lock_proof::validate_asset_lock_transaction_structure::validate_asset_lock_transaction_structure;

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
///
/// The shielded pooled drain counts candidates against it before a
/// transaction exists; a built transaction is judged by
/// [`input_cap_violation`].
#[cfg(feature = "shielded")]
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
/// The cap itself is judged by DPP's own structure validator for the
/// transaction (`validate_asset_lock_transaction_structure`, the same check
/// Platform consensus runs on submission), and only its too-many-inputs
/// verdict is projected onto the wallet's error. The wallet owns no second
/// definition of the bound. A funded build asks before the lock is tracked
/// or sent (`build.rs`), and a resume of a tracked `Built` / `Broadcast` row
/// asks before any re-send (`sync/recovery.rs`): a row built by a version
/// before the pre-broadcast check, or by the build-only API, is otherwise
/// re-sent on every resume, dropped silently by every peer, and never
/// settles. `credit_output_index` is the lock's credit output.
///
/// Any other verdict of that validator is not this question and is left to
/// the submission layer; a validator the platform version does not know is
/// reported and read as no verdict, which sends nothing new and abandons
/// nothing.
pub(crate) fn input_cap_violation(
    transaction: &Transaction,
    credit_output_index: u32,
    platform_version: &dpp::version::PlatformVersion,
) -> Option<PlatformWalletError> {
    let result = match validate_asset_lock_transaction_structure(
        transaction,
        credit_output_index,
        platform_version,
    ) {
        Ok(result) => result,
        Err(error) => {
            tracing::error!(
                %error,
                "asset lock input cap: DPP's structure validator is unavailable for this \
                 platform version; no verdict"
            );
            return None;
        }
    };
    result.errors.iter().find_map(|error| match error {
        ConsensusError::BasicError(BasicError::IdentityAssetLockTransactionTooManyInputsError(
            too_many,
        )) => Some(PlatformWalletError::AssetLockTooManyInputs {
            count: usize::from(too_many.actual_inputs()),
            max: usize::from(too_many.max_inputs()),
        }),
        _ => None,
    })
}
