pub(crate) mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::platform_types::check_tx_proof_verifier::CheckTxProofVerifier;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

use crate::execution::check_tx::CheckTxLevel;

/// What check tx builds for a transition it admits: the execution event fee validation judges,
/// and the bounds among what it reserved on top of the transition's estimate for work a block
/// bills where check tx validated no state (`CheckTxReservation`: a `chooseFirst` scan's budget
/// and its competitors' hashes, a `refersTo` lookup query's allowance). The event's execution
/// operations hold every reserved operation, so fee validation holds the payer to them all;
/// `reserved_bounds` holds the bounds again, so check tx can leave them out of the `gas_wanted`
/// it reports, which Tenderdash fills a block's gas with: a bound is what a block may bill, not
/// what it is expected to.
#[derive(Clone)]
pub(in crate::execution) struct CheckTxExecutionEvent<'a> {
    /// The execution event, the reserved operations among its execution operations.
    pub(in crate::execution) event: ExecutionEvent<'a>,
    /// The bounds reserved on top of the transition's estimate.
    pub(in crate::execution) reserved_bounds: Vec<ValidationOperation>,
}

/// === CHECK TX: NEW ====
/// Full validation for identity create and identity top up
/// Otherwise only validate:
/// * identity has enough balance for fee
/// * identity signature on tx is valid
/// * ST structure is valid
///
/// === CHECK TX: RECHECK ===
/// For identity create and identity top up, make sure asset lock has not been used up
/// For other state transitions verify that the user still has enough balance
///
pub(in crate::execution) fn state_transition_to_execution_event_for_check_tx<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    state_transition: &StateTransition,
    check_tx_level: CheckTxLevel,
    proof_verifier: &CheckTxProofVerifier,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Option<CheckTxExecutionEvent<'a>>>, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transition_to_execution_event_for_check_tx
    {
        0 => v0::state_transition_to_execution_event_for_check_tx_v0(
            platform,
            state_transition,
            check_tx_level,
            proof_verifier,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "state_transition_to_execution_event_for_check_tx".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
