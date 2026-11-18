mod v0;

use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::check_tx_reservation::CheckTxReservation;

/// What check tx reserves, on top of the estimated fee of `action`, for the `refersTo` lookups
/// of its creates when it does not validate their state. Document reference validation bills a
/// block, for each lookup a create's references make, the double SHA-256 of a computed key, the
/// lookup query, at what it reads, and for a lookup declaring `consume` the delete of the
/// document it finds, which the block prices with the create. None of them is in the estimate
/// of a create whose state was not validated, so the reservation holds the signer to them: a
/// signer check tx admits can pay for the create's lookups in a block. A query's cost is only
/// known from state, so it is reserved as an allowance
/// (`FeeDocumentReferenceVersion::lookup_query_credits_reserved`).
///
/// # Parameters
/// * `action`: The transformed state transition whose creates are reserved for.
/// * `drive`: Prices the delete of a consumed document, by estimation.
/// * `block_info`: The block check tx estimates in.
/// * `platform_version`: Selects the method version and prices the reservation.
///
/// # Returns
/// * `Ok(CheckTxReservation)`: the operations reserving the lookups of its creates, none for an
///   action holding no create whose references make one: the keys' hashes and the consumed
///   documents' deletes are billed, the queries' allowances bounds.
/// * `Err(Error)` when the method version is unknown, or pricing a delete fails.
pub(in crate::execution::validation::state_transition) fn reference_lookup_reservation(
    action: &StateTransitionAction,
    drive: &Drive,
    block_info: &BlockInfo,
    platform_version: &PlatformVersion,
) -> Result<CheckTxReservation, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .reference_lookup_reservation
    {
        0 => v0::reference_lookup_reservation_v0(action, drive, block_info, platform_version),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "reference_lookup_reservation".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
