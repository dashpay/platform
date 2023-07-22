use dpp::document::action_type::DocumentActionType;
use dpp::identifier::Identifier;
use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::DataTriggerBinding;
use dpp::version::PlatformVersion;
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::data_trigger_binding::v0::{DataTriggerBindingV0, DataTriggerBindingV0Getters};
use crate::execution::validation::state_transition::documents_batch::data_triggers::{DataTriggerExecutionContext, DataTriggerExecutionResult};

mod v0;

pub enum DataTriggerBinding {
    V0(DataTriggerBindingV0),
}

impl DataTriggerBindingV0Getters for DataTriggerBinding {
    fn execute(
        &self,
        document_transition: &DocumentTransitionAction,
        context: &DataTriggerExecutionContext<'_>,
        platform_version: &PlatformVersion,
    ) -> DataTriggerExecutionResult {
        match self {
            DataTriggerBinding::V0(binding) => {
                binding.execute(document_transition, context, platform_version)
            }
        }
    }

    fn is_matching(
        &self,
        data_contract_id: &Identifier,
        document_type: &str,
        transition_action: DocumentActionType,
    ) -> bool {
        match self {
            DataTriggerBinding::V0(binding) => {
                binding.is_matching(data_contract_id, document_type, transition_action)
            }
        }
    }
}
