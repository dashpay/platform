use crate::execution::validation::state_transition::documents_batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;

use dpp::state_transition::documents_batch_transition::document_transition::action_type::TransitionActionTypeGetter;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use dpp::version::PlatformVersion;
use drive::error::drive::DriveError;
use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::data_trigger_binding::DataTriggerBinding;
use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::data_trigger_binding::DataTriggerBindingV0Getters;
use crate::error::Error;
use crate::error::execution::ExecutionError;

pub trait DataTriggerExecutor {
    fn validate_with_data_triggers(
        &self,
        data_trigger_bindings: &[DataTriggerBinding],
        context: &DataTriggerExecutionContext<'_>,
        platform_version: &PlatformVersion,
    ) -> Result<DataTriggerExecutionResult, Error>;
}

impl DataTriggerExecutor for DocumentTransitionAction {
    fn validate_with_data_triggers(
        &self,
        data_trigger_bindings: &[DataTriggerBinding],
        context: &DataTriggerExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<DataTriggerExecutionResult, Error> {
        let data_contract_id = self
            .base()
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "expecting action to have a base",
            )))?
            .data_contract_id();
        let document_type_name = self
            .base()
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "expecting action to have a base",
            )))?
            .document_type_name();
        let transition_action = self.action_type();

        // Match data triggers by action type, contract ID and document type name
        // and then execute matched triggers until one of them returns invalid result
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
