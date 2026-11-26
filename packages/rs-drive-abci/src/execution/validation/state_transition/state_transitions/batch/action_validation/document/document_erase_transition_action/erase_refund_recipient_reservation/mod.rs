mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

/// What check tx reserves, on top of the estimated fee of a batch whose state it does not
/// validate, for one erase in it: the work the erase's refunds cause for other identities,
/// which erase state validation bills in the block on every path an erase takes. Without it,
/// a signer covering an erase's estimated fee but not that work would be admitted, then
/// refused unpaid in a block, its nonce left to replay the work.
///
/// # Parameters
/// * `drive`: Prices the work.
/// * `epoch`: The epoch whose fee schedule prices it, the one check tx estimates the batch in.
/// * `platform_version`: Selects the method version.
///
/// # Returns
/// * `Ok(ValidationOperation)`: the operation reserving the work of one erase.
/// * `Err(Error)` when the method version is unknown or not active, or pricing the work fails.
pub(in crate::execution::validation::state_transition) fn erase_refund_recipient_reservation(
    drive: &Drive,
    epoch: &Epoch,
    platform_version: &PlatformVersion,
) -> Result<ValidationOperation, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .document_erase_transition_refund_recipient_reservation
    {
        Some(0) => v0::erase_refund_recipient_reservation_v0(drive, epoch, platform_version),
        Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "erase_refund_recipient_reservation".to_string(),
            known_versions: vec![0],
            received: version,
        })),
        None => Err(Error::Execution(ExecutionError::VersionNotActive {
            method: "erase_refund_recipient_reservation".to_string(),
            known_versions: vec![0],
        })),
    }
}
