pub use data_trigger_execution_context::*;
pub use data_trigger_execution_result::*;
pub use reject_data_trigger::*;

use crate::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use crate::consensus::state::data_trigger::data_trigger_execution_error::DataTriggerExecutionError;
use crate::document::action_type::DocumentActionType;
use crate::document::document_transition::DocumentTransition;
use crate::{get_from_transition, prelude::Identifier, state_repository::StateRepositoryLike};

use self::dashpay_data_triggers::create_contact_request_data_trigger;
use self::dpns_triggers::create_domain_data_trigger;
use self::feature_flags_data_triggers::create_feature_flag_data_trigger;
use self::reward_share_data_triggers::create_masternode_reward_shares_data_trigger;
use self::withdrawals_data_triggers::delete_withdrawal_data_trigger;

mod data_trigger_execution_context;

pub mod dashpay_data_triggers;
pub mod dpns_triggers;
pub mod feature_flags_data_triggers;
pub mod get_data_triggers_factory;
pub mod reward_share_data_triggers;
pub mod withdrawals_data_triggers;

mod data_trigger_execution_result;
mod reject_data_trigger;

#[derive(Debug, Clone, Copy, Default)]
pub enum DataTriggerKind {
    CreateDataContractRequest,
    DataTriggerCreateDomain,
    DataTriggerRewardShare,
    DataTriggerReject,
    #[default]
    CrateFeatureFlag,
    DeleteWithdrawal,
}

impl From<DataTriggerKind> for &str {
    fn from(value: DataTriggerKind) -> Self {
        match value {
            DataTriggerKind::CrateFeatureFlag => "createFeatureFlag",
            DataTriggerKind::DataTriggerReject => "dataTriggerReject",
            DataTriggerKind::DataTriggerRewardShare => "dataTriggerRewardShare",
            DataTriggerKind::DataTriggerCreateDomain => "dataTriggerCreateDomain",
            DataTriggerKind::CreateDataContractRequest => "createDataContractRequest",
            DataTriggerKind::DeleteWithdrawal => "deleteWithdrawal",
        }
    }
}

#[derive(Default, Clone)]
pub struct DataTrigger {
    pub data_contract_id: Identifier,
    pub document_type: String,
    pub data_trigger_kind: DataTriggerKind,
    pub transition_action: DocumentActionType,
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

        let maybe_execution_result = execute_trigger(
            self.data_trigger_kind,
            document_transition,
            context,
            self.top_level_identity.as_ref(),
        )
        .await;

        match maybe_execution_result {
            Err(err) => {
                let consensus_error = DataTriggerExecutionError::new(
                    data_contract_id,
                    *get_from_transition!(document_transition, id),
                    err.to_string(),
                );

                result.add_error(consensus_error.into());
                result
            }

            Ok(execution_result) => execution_result,
        }
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
            create_contact_request_data_trigger(document_transition, context, identifier).await
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
        DataTriggerKind::DeleteWithdrawal => {
            delete_withdrawal_data_trigger(document_transition, context, identifier).await
        }
    }
}

fn create_error<SR>(
    context: &DataTriggerExecutionContext<SR>,
    transition_id: Identifier,
    msg: String,
) -> DataTriggerError
where
    SR: StateRepositoryLike,
{
    DataTriggerConditionError::new(context.data_contract.id(), transition_id, msg).into()
}
