use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use dpp::version::PlatformVersion;
use crate::execution::validation::state_transition::batch::data_triggers::triggers::withdrawals::v0::delete_withdrawal_data_trigger_v0;

mod v0;

pub fn delete_withdrawal_data_trigger(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .data_triggers
        .triggers
        .delete_withdrawal_data_trigger
    {
        0 => delete_withdrawal_data_trigger_v0(document_transition, context, platform_version),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "delete_withdrawal_data_trigger".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
