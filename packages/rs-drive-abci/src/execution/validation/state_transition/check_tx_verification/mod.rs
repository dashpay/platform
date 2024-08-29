pub(crate) mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

use crate::execution::check_tx::CheckTxLevel;

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
    state_transition: StateTransition,
    check_tx_level: CheckTxLevel,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Option<ExecutionEvent<'a>>>, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transition_to_execution_event_for_check_tx
    {
        0 => v0::state_transition_to_execution_event_for_check_tx_v0(
            platform,
            state_transition,
            check_tx_level,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "state_transition_to_execution_event_for_check_tx".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
