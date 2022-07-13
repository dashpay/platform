mod data_trigger_execution_context;

pub mod dashpay_data_triggers;
pub mod dpns_triggers;
pub mod feature_flags_data_triggers;
pub mod get_data_triggers_factory;
pub mod reward_share_data_triggers;

mod data_trigger_execution_result;
mod reject_data_trigger;
use futures::future::LocalBoxFuture;

pub use data_trigger_execution_context::*;
pub use data_trigger_execution_result::*;
pub use reject_data_trigger::*;

use crate::document::document_transition::{Action, DocumentCreateTransition, DocumentTransition};
use crate::{
    errors::DataTriggerError, get_from_transition, prelude::Identifier,
    state_repository::StateRepositoryLike,
};

use self::dashpay_data_triggers::create_contract_request_data_trigger;
use self::dpns_triggers::create_domain_data_trigger;
use self::feature_flags_data_triggers::create_feature_flag_data_trigger;
use self::reward_share_data_triggers::create_masternode_reward_shares_data_trigger;

pub type BoxedTrigger<'a, SR> = Box<Trigger<'a, SR>>;
pub type Trigger<'a, SR> =
    dyn Fn(
        &'a DocumentTransition,
        &'a DataTriggerExecutionContext<SR>,
        Option<&'a Identifier>,
    ) -> LocalBoxFuture<'a, Result<DataTriggerExecutionResult, anyhow::Error>>;

#[derive(Debug, Clone, Copy)]
pub enum DataTriggerKind {
    CreateDataContractRequest,
    DataTriggerCreateDomain,
    DataTriggerRewardShare,
    DataTriggerReject,
    CrateFeatureFlag,
}

pub struct DataTrigger {
    pub data_contract_id: Identifier,
    pub document_type: String,
    pub data_trigger_kind: DataTriggerKind,
    pub transition_action: Action,
    pub top_level_identity: Option<Identifier>,
}

impl DataTrigger {
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

    pub async fn execute<'a, SR>(
        &self,
        document_transition: &DocumentTransition,
        context: &DataTriggerExecutionContext<'a, SR>,
    ) -> DataTriggerExecutionResult
    where
        SR: StateRepositoryLike,
    {
        let mut result = DataTriggerExecutionResult::default();
        // TODO remove the clone
        let data_contract_id = context.data_contract.id.to_owned();

        let execution_result = execute_trigger(
            self.data_trigger_kind,
            document_transition,
            context,
            self.top_level_identity.as_ref(),
        )
        .await;

        if let Err(err) = execution_result {
            let consensus_error = DataTriggerError::DataTriggerExecutionError {
                data_contract_id,
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

async fn execute_trigger<'a, SR>(
    trigger_kind: DataTriggerKind,
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<'a, SR>,
    identifier: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
{
    match trigger_kind {
        DataTriggerKind::CreateDataContractRequest => {
            create_contract_request_data_trigger(document_transition, context, identifier).await
        }
        DataTriggerKind::DataTriggerCreateDomain => {
            create_domain_data_trigger(document_transition, context, identifier).await
        }
        DataTriggerKind::CrateFeatureFlag => {
            create_feature_flag_data_trigger(document_transition, context, identifier).await
        }
        DataTriggerKind::DataTriggerReject => {
            reject_data_trigger(document_transition, context, identifier).await
        }
        DataTriggerKind::DataTriggerRewardShare => {
            create_masternode_reward_shares_data_trigger(document_transition, context, identifier)
                .await
        }
    }
}

fn create_error<'a, SR>(
    context: &DataTriggerExecutionContext<'a, SR>,
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
