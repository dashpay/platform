use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::DataTriggerBinding;
use crate::execution::validation::state_transition::documents_batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::ProtocolError;
use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::data_trigger_binding::DataTriggerBinding;

pub trait DataTriggerExecutor {
    fn validate_with_data_triggers<'a>(
        document_transitions: &'a [DocumentTransitionAction],
        context: &DataTriggerExecutionContext<'a>,
    ) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError>;
}

impl DataTriggerExecutor for DocumentTransitionAction {
    fn validate_with_data_triggers(
        data_trigger_bindings: Vec<DataTriggerBinding>,
        context: &DataTriggerExecutionContext,
    ) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError> {
        let data_contract_id = &context.data_contract.id;
        let document_type_name = &document_transition.base().document_type_name;
        let transition_action = document_transition.action();

        data_trigger_bindings
            .into_iter()
            .filter(|data_trigger| {
                data_trigger.is_matching(data_contract_id, document_type_name, transition_action)
            })
            .map(|data_trigger| data_trigger.execute(document_transition, context))
            .collect()
    }
}
