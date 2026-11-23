//! What a host does with the operation id it stored for a pooled Core to
//! shielded drain ("Max").
//!
//! A host persists the operation id before it calls the drain, so that a
//! restart can ask what became of it
//! ([`PlatformWallet::shielded_wallet_drain_operation_status`](crate::wallet::PlatformWallet::shielded_wallet_drain_operation_status)).
//! The rules for acting on the answer live here, so every host applies the
//! same ones: the host gathers the facts it owns (whether one of its attempts
//! is running, whether it holds a stored id) and does what these return.

use dashcore::Network;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::wallet::asset_lock::tracked::AssetLockStatus;
use crate::wallet::asset_lock::AssetLockOperationState;

use super::wallet_drain::WalletDrainPreviewStatus;

/// What one status read of a stored operation id produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletDrainStatusRead {
    /// The read succeeded and reported this state.
    Reported(AssetLockOperationState),
    /// The read failed. That is never evidence that nothing was sent.
    Failed,
}

/// What a host does with its stored operation id after a status read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletDrainMarkerDecision {
    /// The operation needs nothing more: remove the stored id.
    Forget,
    /// The operation may still be unfinished: keep the stored id.
    Keep,
}

/// Whether a host keeps the operation id it stored, given one status read.
///
/// `Completed` is done. `NotSent` from a successful durable read means
/// nothing was dispatched, unless one of the host's attempts for this id ran
/// or started while the read was in progress (`attempt_overlapped_read`): that
/// attempt may have dispatched after the read. `Pending` and a failed read
/// keep the id.
pub fn wallet_drain_marker_decision(
    read: WalletDrainStatusRead,
    attempt_overlapped_read: bool,
) -> WalletDrainMarkerDecision {
    match read {
        WalletDrainStatusRead::Reported(AssetLockOperationState::Completed) => {
            WalletDrainMarkerDecision::Forget
        }
        WalletDrainStatusRead::Reported(AssetLockOperationState::NotSent) => {
            if attempt_overlapped_read {
                WalletDrainMarkerDecision::Keep
            } else {
                WalletDrainMarkerDecision::Forget
            }
        }
        WalletDrainStatusRead::Reported(AssetLockOperationState::Pending)
        | WalletDrainStatusRead::Failed => WalletDrainMarkerDecision::Keep,
    }
}

/// The one thing a host offers for a stored operation id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletDrainRecoveryAction {
    /// One of the host's attempts for this operation is running.
    InProgress,
    /// Resume the asset lock the last read named, by its exact outpoint.
    Resume,
    /// Read the status again. Sends nothing.
    CheckAgain,
}

/// What a host offers for a stored operation id.
///
/// A running attempt wins. Otherwise only a `Pending` report that named an
/// asset lock allows a spend, and that spend is a resume of that exact
/// outpoint, which can never build a second lock. Every other case (no read
/// yet, a failed read, `Pending` without a lock, `NotSent`, `Completed`) can
/// only read the status again: the drain is never called again for a stored
/// id, because a repeated call could build a lock while the state is `NotSent`.
pub fn wallet_drain_recovery_action(
    attempt_in_flight: bool,
    last_read: Option<WalletDrainStatusRead>,
    read_named_asset_lock: bool,
) -> WalletDrainRecoveryAction {
    if attempt_in_flight {
        return WalletDrainRecoveryAction::InProgress;
    }
    match last_read {
        Some(WalletDrainStatusRead::Reported(AssetLockOperationState::Pending))
            if read_named_asset_lock =>
        {
            WalletDrainRecoveryAction::Resume
        }
        _ => WalletDrainRecoveryAction::CheckAgain,
    }
}

/// Whether a returned drain attempt needs nothing more.
///
/// Only `Completed` does, and it does even when the call also returned an
/// error (a repeated operation id reports "already consumed" beside it).
/// `Pending` and `NotSent` are shown as a failure of that attempt, with the
/// error the call returned.
pub fn wallet_drain_attempt_finished(state: AssetLockOperationState) -> bool {
    state == AssetLockOperationState::Completed
}

/// Whether a host may start a drain after this preview.
///
/// Only `InsufficientFunds` refuses: there is nothing to shield.
/// `InputLimitMayBeExceeded` does not, because the preview counts inputs
/// other operations hold and the execution may still fit.
pub fn wallet_drain_preview_allows_start(status: WalletDrainPreviewStatus) -> bool {
    match status {
        WalletDrainPreviewStatus::Ready | WalletDrainPreviewStatus::InputLimitMayBeExceeded => true,
        WalletDrainPreviewStatus::InsufficientFunds => false,
    }
}

/// Whether a host offers to start a new drain on `network`.
///
/// Not on mainnet until a drain's completion is proof-backed
/// (dashpay/platform#5580): a drain can be recorded as completed from an
/// unauthenticated "already consumed" report. Recovering a drain that already
/// exists is offered on every network.
pub fn wallet_drain_start_offered(network: Network) -> bool {
    network != Network::Mainnet
}

/// Where a tracked asset lock that carries an operation id is listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletDrainLockListing {
    /// Not decided here: the host's rule for resumable locks applies.
    Unaffected,
    /// The host's stored-operation row owns this lock: list it nowhere else.
    Hidden,
    /// List it with the resumable locks, although it is only `Built`.
    Offered,
}

/// Where a host lists a tracked asset lock, as far as drains are concerned.
///
/// A lock without an operation id is not a drain's. A lock whose operation
/// id the host holds a stored id for is shown by that stored operation alone.
/// A drain's lock with no stored id (a lost marker, another caller) is
/// reachable only by a resume of its exact outpoint, so it is offered even
/// while it is still the `Built` checkpoint, which a host otherwise hides as
/// a build in flight.
pub fn wallet_drain_lock_listing(
    funding_type: AssetLockFundingType,
    status: AssetLockStatus,
    has_operation_id: bool,
    host_holds_stored_id: bool,
) -> WalletDrainLockListing {
    if !has_operation_id {
        return WalletDrainLockListing::Unaffected;
    }
    if host_holds_stored_id {
        return WalletDrainLockListing::Hidden;
    }
    if funding_type == AssetLockFundingType::AssetLockShieldedAddressTopUp
        && status == AssetLockStatus::Built
    {
        return WalletDrainLockListing::Offered;
    }
    WalletDrainLockListing::Unaffected
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATES: [AssetLockOperationState; 3] = [
        AssetLockOperationState::NotSent,
        AssetLockOperationState::Pending,
        AssetLockOperationState::Completed,
    ];

    #[test]
    fn should_forget_a_stored_id_only_when_the_operation_needs_nothing_more() {
        use WalletDrainMarkerDecision::{Forget, Keep};
        use WalletDrainStatusRead::{Failed, Reported};
        for (read, overlapped, expected) in [
            (Reported(AssetLockOperationState::Completed), false, Forget),
            (Reported(AssetLockOperationState::Completed), true, Forget),
            (Reported(AssetLockOperationState::NotSent), false, Forget),
            (Reported(AssetLockOperationState::NotSent), true, Keep),
            (Reported(AssetLockOperationState::Pending), false, Keep),
            (Reported(AssetLockOperationState::Pending), true, Keep),
            (Failed, false, Keep),
            (Failed, true, Keep),
        ] {
            assert_eq!(
                wallet_drain_marker_decision(read, overlapped),
                expected,
                "{read:?}, overlapped: {overlapped}"
            );
        }
    }

    #[test]
    fn should_offer_a_resume_only_for_a_pending_report_that_named_an_asset_lock() {
        use WalletDrainRecoveryAction::{CheckAgain, InProgress, Resume};
        let reads = [
            None,
            Some(WalletDrainStatusRead::Failed),
            Some(WalletDrainStatusRead::Reported(
                AssetLockOperationState::NotSent,
            )),
            Some(WalletDrainStatusRead::Reported(
                AssetLockOperationState::Pending,
            )),
            Some(WalletDrainStatusRead::Reported(
                AssetLockOperationState::Completed,
            )),
        ];
        for read in reads {
            for named in [false, true] {
                assert_eq!(
                    wallet_drain_recovery_action(true, read, named),
                    InProgress,
                    "a running attempt wins: {read:?}, named: {named}"
                );
                let pending_with_lock = named
                    && read
                        == Some(WalletDrainStatusRead::Reported(
                            AssetLockOperationState::Pending,
                        ));
                assert_eq!(
                    wallet_drain_recovery_action(false, read, named),
                    if pending_with_lock {
                        Resume
                    } else {
                        CheckAgain
                    },
                    "{read:?}, named: {named}"
                );
            }
        }
    }

    #[test]
    fn should_count_only_a_completed_attempt_as_finished() {
        for state in STATES {
            assert_eq!(
                wallet_drain_attempt_finished(state),
                state == AssetLockOperationState::Completed
            );
        }
    }

    #[test]
    fn should_refuse_a_start_only_when_the_preview_has_nothing_to_shield() {
        assert!(wallet_drain_preview_allows_start(
            WalletDrainPreviewStatus::Ready
        ));
        assert!(wallet_drain_preview_allows_start(
            WalletDrainPreviewStatus::InputLimitMayBeExceeded
        ));
        assert!(!wallet_drain_preview_allows_start(
            WalletDrainPreviewStatus::InsufficientFunds
        ));
    }

    #[test]
    fn should_not_offer_a_new_drain_on_mainnet() {
        assert!(!wallet_drain_start_offered(Network::Mainnet));
        for network in [Network::Testnet, Network::Devnet, Network::Regtest] {
            assert!(wallet_drain_start_offered(network), "{network:?}");
        }
    }

    #[test]
    fn should_list_a_drain_lock_by_its_stored_id_or_as_a_resumable_checkpoint() {
        use WalletDrainLockListing::{Hidden, Offered, Unaffected};
        let shielded = AssetLockFundingType::AssetLockShieldedAddressTopUp;
        let address = AssetLockFundingType::AssetLockAddressTopUp;
        for (funding, status, has_id, stored, expected) in [
            // No operation id: never a drain's lock, whatever the host holds.
            (shielded, AssetLockStatus::Built, false, false, Unaffected),
            (shielded, AssetLockStatus::Built, false, true, Unaffected),
            (
                shielded,
                AssetLockStatus::Broadcast,
                false,
                false,
                Unaffected,
            ),
            // A stored id owns the lock at every status.
            (shielded, AssetLockStatus::Built, true, true, Hidden),
            (shielded, AssetLockStatus::Broadcast, true, true, Hidden),
            (shielded, AssetLockStatus::ChainLocked, true, true, Hidden),
            (
                shielded,
                AssetLockStatus::DrainCompleted,
                true,
                true,
                Hidden,
            ),
            // No stored id: the Built checkpoint is offered, later statuses
            // follow the host's own rule.
            (shielded, AssetLockStatus::Built, true, false, Offered),
            (
                shielded,
                AssetLockStatus::Broadcast,
                true,
                false,
                Unaffected,
            ),
            (
                shielded,
                AssetLockStatus::DrainCompleted,
                true,
                false,
                Unaffected,
            ),
            (address, AssetLockStatus::Built, true, false, Unaffected),
        ] {
            assert_eq!(
                wallet_drain_lock_listing(funding, status.clone(), has_id, stored),
                expected,
                "{funding:?}, {status:?}, has id: {has_id}, stored: {stored}"
            );
        }
    }
}
