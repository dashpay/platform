//! Asset lock lifecycle management.
//!
//! Tracks asset lock transactions from build through finality (IS/CL) and
//! Platform consumption. Shared across sub-wallets via `Arc<AssetLockManager>`.

use dashcore::Transaction;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::asset_lock_proof::validate_asset_lock_transaction_inputs::validate_asset_lock_transaction_inputs;

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

pub use build::{AssetLockBuildAmount, AssetLockBuiltObserver};
pub use lock_notify_handler::LockNotifyHandler;
pub use orchestration::AssetLockFunding;

/// The asset-lock input cap this protocol version holds an instant proof to
/// (`max_asset_lock_transaction_inputs`, 100 since protocol version 11).
/// Dash Core's relay policy refuses the same bound
/// (`MAX_STANDARD_ASSET_LOCK_INPUTS`, `assetlocktx-too-many-inputs`), and
/// peers drop such a lock silently, so every broadcasting build refuses it
/// before anything reaches the wire. The build-only
/// `build_asset_lock_transaction*` does not check it: refusing it at
/// construction belongs in key-wallet's builder (dashpay/rust-dashcore#1190).
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
/// The cap itself is judged by DPP's own input check for the transaction
/// (`validate_asset_lock_transaction_inputs`), and its verdict is projected
/// onto the wallet's error. The wallet owns no second definition of the
/// bound. It asks that check rather than the transaction structure check:
/// from protocol 15 the structure check no longer caps inputs, as a chain
/// proof's transaction is not carried in the state transition, while Dash
/// Core still refuses to relay a larger lock whichever proof would follow. A funded build asks before the lock is tracked or sent
/// (`build.rs`), and a resume of a tracked `Built` / `Broadcast` row asks
/// before any re-send (`sync/recovery.rs`): a row built by a version before
/// the pre-broadcast check, or by the build-only API, is otherwise re-sent on
/// every resume, dropped silently by every peer, and never settles.
pub(crate) fn input_cap_violation(
    transaction: &Transaction,
    platform_version: &dpp::version::PlatformVersion,
) -> Option<PlatformWalletError> {
    let result = validate_asset_lock_transaction_inputs(transaction, platform_version);
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::tests::fixtures::instant_asset_lock_proof_transaction_fixture_with_inputs;
    use dpp::version::PlatformVersion;

    /// Dash Core does not relay an asset lock over the input cap, whether or not Platform could
    /// redeem it with a chain proof, so the wallet keeps refusing to send one at the latest
    /// protocol version, where the cap holds only instant proofs.
    #[test]
    fn should_refuse_an_asset_lock_over_the_input_cap_at_the_latest_protocol_version() {
        let over_cap = instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);
        let at_cap = instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 100);

        match input_cap_violation(&over_cap, PlatformVersion::latest()) {
            Some(PlatformWalletError::AssetLockTooManyInputs { count, max }) => {
                assert_eq!(count, 101);
                assert_eq!(max, 100);
            }
            other => panic!("expected AssetLockTooManyInputs, got {other:?}"),
        }
        assert!(input_cap_violation(&at_cap, PlatformVersion::latest()).is_none());
    }
}
