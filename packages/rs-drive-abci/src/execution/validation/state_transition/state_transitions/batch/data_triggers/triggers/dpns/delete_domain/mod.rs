use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::batch::data_triggers::triggers::dpns::delete_domain::v0::delete_domain_data_trigger_v0;
use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

mod v0;

/// The DPNS `domain` delete trigger, bound from protocol version 15 (data trigger
/// bindings 3), when DPNS v3 lets a name's owner delete it.
pub fn delete_domain_data_trigger(
    document_transition: &DocumentTransitionAction,
    context: &mut DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .data_triggers
        .triggers
        .delete_domain_data_trigger
    {
        Some(0) => delete_domain_data_trigger_v0(document_transition, context, platform_version),
        Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "delete_domain_data_trigger".to_string(),
            known_versions: vec![0],
            received: version,
        })),
        None => Err(Error::Execution(ExecutionError::VersionNotActive {
            method: "delete_domain_data_trigger".to_string(),
            known_versions: vec![0],
        })),
    }
}
