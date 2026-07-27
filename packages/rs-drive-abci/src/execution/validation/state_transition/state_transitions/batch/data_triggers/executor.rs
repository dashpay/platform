use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionTypeGetter;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use dpp::version::PlatformVersion;
use crate::execution::validation::state_transition::batch::data_triggers::bindings::data_trigger_binding::DataTriggerBinding;
use crate::execution::validation::state_transition::batch::data_triggers::bindings::data_trigger_binding::DataTriggerBindingV0Getters;
use crate::error::Error;

pub trait DataTriggerExecutor {
    fn validate_with_data_triggers(
        &self,
        data_trigger_bindings: &[DataTriggerBinding],
        context: &mut DataTriggerExecutionContext<'_>,
        platform_version: &PlatformVersion,
    ) -> Result<DataTriggerExecutionResult, Error>;
}

impl DataTriggerExecutor for DocumentTransitionAction {
    fn validate_with_data_triggers(
        &self,
        data_trigger_bindings: &[DataTriggerBinding],
        context: &mut DataTriggerExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<DataTriggerExecutionResult, Error> {
        let data_contract_id = self.base().data_contract_id();
        let document_type_name = self.base().document_type_name();
        let transition_action = self.action_type();

        // Match data triggers by action type, contract ID and document
        // type name, then execute matched triggers until one returns
        // invalid. `_v1` triggers bill their own drive reads directly
        // via `context.state_transition_execution_context.add_operation`;
        // `_v0` triggers don't bill (PROTOCOL_VERSION_11 chain replay).
        for data_trigger_binding in data_trigger_bindings {
            if !data_trigger_binding.is_matching(
                &data_contract_id,
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
