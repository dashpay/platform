mod data_trigger_execution_context;
use std::future::Future;
use std::pin::Pin;

pub mod dashpay_data_triggers;
mod data_trigger_execution_result;
mod reject_data_trigger;

pub use data_trigger_execution_context::*;
pub use data_trigger_execution_result::*;
pub use reject_data_trigger::*;

use crate::{
    errors::DataTriggerError, get_from_transition, prelude::Identifier,
    state_repository::StateRepositoryLike,
};

use crate::document::document_transition::{Action, DocumentTransition};

pub type Trigger<SR> =
    fn(
        &DocumentTransition,
        &DataTriggerExecutionContext<SR>,
        Option<&Identifier>,
    ) -> Pin<Box<dyn Future<Output = Result<DataTriggerExecutionResult, anyhow::Error>>>>;

pub struct DataTrigger<SR>
where
    SR: StateRepositoryLike,
{
    pub data_contract_id: Identifier,
    pub document_type: String,
    pub trigger: Trigger<SR>,
    pub transition_action: Action,
    pub top_level_identity: Option<Identifier>,
}

impl<SR> DataTrigger<SR>
where
    SR: StateRepositoryLike,
{
    /// Check this trigger is matching for specified data
    pub fn is_matching_trigger_for_data(
        &self,
        data_contract_id: Identifier,
        document_type: String,
        transition_action: Action,
    ) -> bool {
        self.data_contract_id == data_contract_id
            && self.document_type == document_type
            && self.transition_action == transition_action
    }

    pub async fn execute(
        &self,
        document_transition: &DocumentTransition,
        context: &DataTriggerExecutionContext<SR>,
    ) -> DataTriggerExecutionResult
    where
        SR: StateRepositoryLike,
    {
        // not sure what the function may contain
        let mut result = DataTriggerExecutionResult::default();

        let execution_result = (self.trigger)(
            document_transition,
            context,
            self.top_level_identity.as_ref(),
        )
        .await;

        if let Err(err) = execution_result {
            let consensus_error = DataTriggerError::DataTriggerExecutionError {
                data_contract_id: context.data_contract.id.clone(),
                document_transition_id: get_from_transition!(document_transition, id).clone(),
                message: err.to_string(),
                execution_error: err,
                document_transition: None,
                owner_id: None,
            };
            result.add_error(consensus_error.into());
            return result;
        }

        result
    }
}
