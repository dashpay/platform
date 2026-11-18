mod v0;
mod v1;

use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;

/// What check tx reserves, on top of the estimated fee of `action`, for the `chooseFirst`
/// checks of its creates when it does not validate their state: for every create of a
/// document type declaring `chooseFirst` (registration admits at most one per type), the
/// double SHA-256s of its `findBy` and `validWhen` preimages, priced by the blocks the
/// create's own values make them (a transient param counts, though no stored document keeps
/// it), and the competitor scan's budget (`choose_first_scan_budget`). From generation 1 also
/// the `validWhen` hash of each competitor the scan may read, which document reference
/// validation 1 bills in full. Nothing for anything else, nor for a create whose preimage
/// cannot be assembled, which structure validation refuses before any read. Without it, a
/// signer covering the create's estimated fee but not these would be admitted, then refused
/// unpaid in a block, its nonce left to replay the work.
///
/// # Parameters
/// * `action`: The transformed state transition whose creates are reserved for.
/// * `platform_version`: Selects the method version and prices the reservation.
///
/// # Returns
/// * `Ok(Vec<ValidationOperation>)`: the operations reserving the `chooseFirst` checks of its
///   creates, none for an action holding no such create.
/// * `Err(Error)` when the method version is unknown, or the reservation overflows.
pub(in crate::execution::validation::state_transition) fn choose_first_reservation(
    action: &StateTransitionAction,
    platform_version: &PlatformVersion,
) -> Result<Vec<ValidationOperation>, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .choose_first_reservation
    {
        0 => Ok(v0::choose_first_reservation_v0(action, platform_version)),
        1 => v1::choose_first_reservation_v1(action, platform_version),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "choose_first_reservation".to_string(),
            known_versions: vec![0, 1],
            received: version,
        })),
    }
}
