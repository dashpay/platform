use futures::future::LocalBoxFuture;
mod data_trigger_execution_context;

pub mod dashpay_data_triggers;
pub mod dpns_triggers;
pub mod feature_flags_data_triggers;
pub mod get_data_triggers_factory;
pub mod reward_share_data_triggers;

mod data_trigger_execution_result;
mod reject_data_trigger;

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

#[derive(Debug, Clone, Copy)]
pub enum DataTriggerCode {
    CreateDataContractRequest,
    DataTriggerCreateDomain,
    DataTriggerRewardShare,
    DataTriggerReject,
    CrateFeatureFlag,
}

pub struct DataTrigger {
    pub data_contract_id: Identifier,
    pub document_type: String,
    pub data_trigger_code: DataTriggerCode,
    pub transition_action: Action,
    pub top_level_identity: Option<Identifier>,
}

impl DataTrigger {
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

    pub async fn execute<SR>(
        &self,
        document_transition: &DocumentTransition,
        context: &DataTriggerExecutionContext<SR>,
    ) -> DataTriggerExecutionResult
    where
        SR: StateRepositoryLike,
    {
        let mut result = DataTriggerExecutionResult::default();
        let execution_result = trigger_executor(
            self.data_trigger_code,
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

pub async fn trigger_executor<SR>(
    trigger_code: DataTriggerCode,
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<SR>,
    identifier: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
{
    match trigger_code {
        DataTriggerCode::CreateDataContractRequest => {
            create_contract_request_data_trigger(document_transition, context, identifier).await
        }
        DataTriggerCode::DataTriggerCreateDomain => {
            create_domain_data_trigger(document_transition, context, identifier).await
        }

        DataTriggerCode::CrateFeatureFlag => {
            create_feature_flag_data_trigger(document_transition, context, identifier).await
        }
        DataTriggerCode::DataTriggerReject => {
            reject_data_trigger(document_transition, context, identifier).await
        }
        DataTriggerCode::DataTriggerRewardShare => {
            create_masternode_reward_shares_data_trigger(document_transition, context, identifier)
                .await
        }
    }
}
