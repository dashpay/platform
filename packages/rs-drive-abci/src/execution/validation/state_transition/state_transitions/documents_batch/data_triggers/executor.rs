use crate::execution::validation::state_transition::documents_batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::ProtocolError;
use dpp::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use dpp::version::PlatformVersion;
use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::data_trigger_binding::DataTriggerBinding;
use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::data_trigger_binding::DataTriggerBindingV0Getters;

pub trait DataTriggerExecutor {
    fn validate_with_data_triggers<'a>(
        &self,
        data_trigger_bindings: &Vec<DataTriggerBinding>,
        context: &DataTriggerExecutionContext<'a>,
        platform_version: &PlatformVersion,
    ) -> Result<DataTriggerExecutionResult, ProtocolError>;
}

impl<'a> DataTriggerExecutor for DocumentTransitionAction<'a> {
    fn validate_with_data_triggers(
        &self,
        data_trigger_bindings: &Vec<DataTriggerBinding>,
        context: &DataTriggerExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<DataTriggerExecutionResult, ProtocolError> {
        let data_contract_id = &context.data_contract.id();
        let document_type_name = self.base().document_type_name();
        let transition_action = self.action_type();

        // Match data triggers by action type, contract ID and document type name
        // and then execute matched triggers until one of them returns invalid result
        for data_trigger_binding in data_trigger_bindings {
            if !data_trigger_binding.is_matching(
                data_contract_id,
                document_type_name,
                transition_action,
            ) {
                continue;
            }

            let result = data_trigger_binding.execute(self, context, platform_version)?;

            if !result.is_valid() {
                return Ok(result);
            }
        }

        Ok(DataTriggerExecutionResult::default())
    }
}
