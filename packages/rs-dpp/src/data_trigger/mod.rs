use futures::future::LocalBoxFuture;

pub use data_trigger_execution_context::*;
pub use data_trigger_execution_result::*;
pub use reject_data_trigger::*;

use crate::document::document_transition::{Action, DocumentCreateTransition, DocumentTransition};
use crate::{
    errors::DataTriggerError, get_from_transition, prelude::Identifier,
    state_repository::StateRepositoryLike,
};

mod data_trigger_execution_context;

pub mod dashpay_data_triggers;
pub mod dpns_triggers;
pub mod feature_flags_data_triggers;
pub mod get_data_triggers_factory;
pub mod reward_share_data_triggers;

mod data_trigger_execution_result;
mod reject_data_trigger;

pub type BoxedTrigger<'a, SR> = Box<Trigger<'a, SR>>;
pub type Trigger<'a, SR> =
    dyn Fn(
        &'a DocumentTransition,
        &'a DataTriggerExecutionContext<SR>,
        Option<&'a Identifier>,
    ) -> LocalBoxFuture<'a, Result<DataTriggerExecutionResult, anyhow::Error>>;

pub struct DataTrigger<'a, SR>
where
    SR: StateRepositoryLike,
{
    pub data_contract_id: Identifier,
    pub document_type: String,
    pub trigger: BoxedTrigger<'a, SR>,
    pub transition_action: Action,
    pub top_level_identity: Option<Identifier>,
}

impl<'a, SR> DataTrigger<'a, SR>
where
    SR: StateRepositoryLike,
{
    /// Check this trigger is matching for specified data
    pub fn is_matching_trigger_for_data(
        &self,
        data_contract_id: &Identifier,
        document_type: &str,
        transition_action: Action,
    ) -> bool {
        &self.data_contract_id == data_contract_id
            && self.document_type == document_type
            && self.transition_action == transition_action
    }

    pub async fn execute(
        &'a self,
        document_transition: &'a DocumentTransition,
        context: &'a DataTriggerExecutionContext<SR>,
    ) -> DataTriggerExecutionResult
    where
        SR: StateRepositoryLike,
    {
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

pub fn new_error<SR>(
    context: &DataTriggerExecutionContext<SR>,
    dt_create: &DocumentCreateTransition,
    msg: String,
) -> DataTriggerError
where
    SR: StateRepositoryLike,
{
    DataTriggerError::DataTriggerConditionError {
        data_contract_id: context.data_contract.id.clone(),
        document_transition_id: dt_create.base.id.clone(),
        message: msg,
        owner_id: Some(context.owner_id.clone()),
        document_transition: Some(DocumentTransition::Create(dt_create.clone())),
    }
}
