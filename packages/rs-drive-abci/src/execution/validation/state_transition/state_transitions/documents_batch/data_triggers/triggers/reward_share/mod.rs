use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::documents_batch::data_triggers::{DataTriggerExecutionContext, DataTriggerExecutionResult};
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::reward_share::v0::create_masternode_reward_shares_data_trigger_v0;

mod v0;

pub fn create_masternode_reward_shares_data_trigger(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .documents_batch_state_transition
        .data_triggers
        .triggers
        .create_masternode_reward_shares_data_trigger
    {
        0 => create_masternode_reward_shares_data_trigger_v0(
            document_transition,
            context,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "create_masternode_reward_shares_data_trigger".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
