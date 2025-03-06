use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use derive_more::From;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::document_transition::{
    DocumentTransitionAction, DocumentTransitionActionType,
};

use crate::error::Error;
pub use v0::*;

mod v0;

#[derive(Clone, From)]
pub enum DataTriggerBinding {
    V0(DataTriggerBindingV0),
}

impl DataTriggerBindingV0Getters for DataTriggerBinding {
    fn execute(
        &self,
        document_transition: &DocumentTransitionAction,
        context: &DataTriggerExecutionContext<'_>,
        platform_version: &PlatformVersion,
    ) -> Result<DataTriggerExecutionResult, Error> {
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
        transition_action_type: DocumentTransitionActionType,
    ) -> bool {
        match self {
            DataTriggerBinding::V0(binding) => {
                binding.is_matching(data_contract_id, document_type, transition_action_type)
            }
        }
    }
}
