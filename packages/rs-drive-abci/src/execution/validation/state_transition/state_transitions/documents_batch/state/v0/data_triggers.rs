use crate::execution::validation::state_transition::documents_batch::data_triggers::{
    data_trigger_bindings_list, DataTriggerExecutionContext,
};
use dpp::consensus::ConsensusError;
use dpp::prelude::ConsensusValidationResult;

fn execute_data_triggers() {
    let data_trigger_execution_context = DataTriggerExecutionContext {
        platform,
        transaction,
        owner_id: &owner_id,
        data_contract,
        state_transition_execution_context: execution_context,
    };

    let data_trigger_bindings = data_trigger_bindings_list(plat)?;

    for document_transition_action in document_transition_actions {
        let data_trigger_execution_result =
            document_transition_action.validate_with_data_triggers(data_trigger_execution_context);

        if !data_trigger_execution_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                execution_result
                    .errors
                    .into_iter()
                    .map(|e| ConsensusError::StateError(e.into()))
                    .collect(),
            ));
        }
    }
}
