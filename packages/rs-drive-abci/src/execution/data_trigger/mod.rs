pub use data_trigger_execution_context::*;
pub use data_trigger_execution_result::*;
use dpp::document::document_transition::{Action, DocumentCreateTransition};
use dpp::get_from_transition;
use dpp::platform_value::Identifier;
use dpp::prelude::DocumentTransition;
use dpp::validation::SimpleValidationResult;
pub use reject_data_trigger::*;
use crate::error::data_trigger::DataTriggerError;
use crate::error::Error;

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

mod reject_data_trigger;

pub type DataTriggerExecutionResult = SimpleValidationResult<DataTriggerError>;

#[derive(Debug, Clone, Copy)]
pub enum DataTriggerKind {
    CreateDataContractRequest,
    DataTriggerCreateDomain,
    DataTriggerRewardShare,
    DataTriggerReject,
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

impl Default for DataTriggerKind {
    fn default() -> Self {
        DataTriggerKind::CrateFeatureFlag
    }
}

#[derive(Default, Clone)]
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

    pub fn execute<'a>(
        &self,
        document_transition: &DocumentTransition,
        context: &DataTriggerExecutionContext<'a>,
    ) -> DataTriggerExecutionResult
    {
        let mut result = DataTriggerExecutionResult::default();
        // TODO remove the clone
        let data_contract_id = context.data_contract.id.to_owned();

        let maybe_execution_result = execute_trigger(
            self.data_trigger_kind,
            document_transition,
            context,
            self.top_level_identity.as_ref(),
        );

        match maybe_execution_result {
            Err(err) => {
                let consensus_error = DataTriggerError::DataTriggerExecutionError {
                    data_contract_id,
                    document_transition_id: *get_from_transition!(document_transition, id),
                    message: err.to_string(),
                    execution_error: err,
                    document_transition: None,
                    owner_id: None,
                };
                result.add_error(consensus_error.into());
                result
            }

            Ok(execution_result) => execution_result,
        }
    }
}

fn execute_trigger<'a>(
    trigger_kind: DataTriggerKind,
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<'a>,
    identifier: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, Error>
{
    match trigger_kind {
        DataTriggerKind::CreateDataContractRequest => {
            create_contact_request_data_trigger(document_transition, context, identifier)
        }
        DataTriggerKind::DataTriggerCreateDomain => {
            create_domain_data_trigger(document_transition, context, identifier)
        }
        DataTriggerKind::CrateFeatureFlag => {
            create_feature_flag_data_trigger(document_transition, context, identifier)
        }
        DataTriggerKind::DataTriggerReject => {
            reject_data_trigger(document_transition, context, identifier)
        }
        DataTriggerKind::DataTriggerRewardShare => {
            create_masternode_reward_shares_data_trigger(document_transition, context, identifier)

        }
        DataTriggerKind::DeleteWithdrawal => {
            delete_withdrawal_data_trigger(document_transition, context, identifier)
        }
    }
}

fn create_error(
    context: &DataTriggerExecutionContext,
    dt_create: &DocumentCreateTransition,
    msg: String,
) -> DataTriggerError
{
    DataTriggerError::DataTriggerConditionError {
        data_contract_id: context.data_contract.id,
        document_transition_id: dt_create.base.id,
        message: msg,
        owner_id: Some(*context.owner_id),
        document_transition: Some(DocumentTransition::Create(dt_create.clone())),
    }
}
