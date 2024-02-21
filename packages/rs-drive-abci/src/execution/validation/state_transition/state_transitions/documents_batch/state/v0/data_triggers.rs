use crate::error::Error;
use crate::execution::validation::state_transition::documents_batch::data_triggers::{
    data_trigger_bindings_list, DataTriggerExecutionContext, DataTriggerExecutionResult,
    DataTriggerExecutor,
};
use dpp::version::PlatformVersion;

use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;

pub(super) fn execute_data_triggers(
    document_transition_actions: &Vec<DocumentTransitionAction>,
    context: &DataTriggerExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_trigger_bindings = data_trigger_bindings_list(platform_version)?;

    for document_transition_action in document_transition_actions {
        if matches!(
            document_transition_action,
            DocumentTransitionAction::BumpIdentityDataContractNonce(_)
        ) {
            continue;
        }
        let data_trigger_execution_result = document_transition_action
            .validate_with_data_triggers(&data_trigger_bindings, context, platform_version)?;

        if !data_trigger_execution_result.is_valid() {
            return Ok(data_trigger_execution_result);
        }
    }

    Ok(DataTriggerExecutionResult::default())
}
