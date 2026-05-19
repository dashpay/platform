use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::batch::data_triggers::triggers::reject::v0::reject_data_trigger_v0;
use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

mod v0;

pub fn reject_data_trigger(
    document_transition: &DocumentTransitionAction,
    _context: &DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<(DataTriggerExecutionResult, FeeResult), Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .data_triggers
        .triggers
        .reject_data_trigger
    {
        // Reject performs no drive reads — FeeResult is always default.
        0 => Ok((reject_data_trigger_v0(document_transition)?, FeeResult::default())),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "reject_data_trigger".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
