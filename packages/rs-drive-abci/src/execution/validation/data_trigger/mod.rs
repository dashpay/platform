use crate::error::Error;
pub use data_trigger_execution_context::*;
use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;
use dpp::document::document_transition::{
    Action, DocumentCreateTransitionAction, DocumentTransitionAction,
};
use dpp::get_from_transition_action;
use dpp::platform_value::Identifier;
use dpp::validation::SimpleValidationResult;
pub use reject_data_trigger::*;

use self::dashpay_data_triggers::create_contact_request_data_trigger;
use self::dpns_triggers::create_domain_data_trigger;
use self::feature_flags_data_triggers::create_feature_flag_data_trigger;
use self::reward_share_data_triggers::create_masternode_reward_shares_data_trigger;
use self::withdrawals_data_triggers::delete_withdrawal_data_trigger;

mod data_trigger_execution_context;

/// The `dashpay_data_triggers` module contains data triggers specific to the DashPay data contract.
pub mod dashpay_data_triggers;

/// The `dpns_triggers` module contains data triggers specific to the DPNS data contract.
pub mod dpns_triggers;

/// The `feature_flags_data_triggers` module contains data triggers related to feature flags.
pub mod feature_flags_data_triggers;

/// The `get_data_triggers_factory` module contains a factory function for creating data triggers.
pub mod get_data_triggers_factory;

/// The `reward_share_data_triggers` module contains data triggers related to reward sharing.
pub mod reward_share_data_triggers;

/// The `withdrawals_data_triggers` module contains data triggers related to withdrawals.
pub mod withdrawals_data_triggers;

mod reject_data_trigger;

/// A type alias for a `SimpleValidationResult` with a `DataTriggerActionError` as the error type.
///
/// This type is used to represent the result of executing a data trigger on the blockchain. It contains either a
/// successful result or a `DataTriggerActionError`, indicating the failure of the trigger.
pub type DataTriggerExecutionResult = SimpleValidationResult<DataTriggerActionError>;

/// An enumeration representing the different kinds of data triggers that can be executed on the blockchain.
///
/// Each variant of the enum corresponds to a specific type of data trigger that can be executed. The enum is used
/// throughout the data trigger system to identify the type of trigger that is being executed.
#[derive(Debug, Clone, Copy, Default)]
pub enum DataTriggerKind {
    /// A data trigger that handles the creation of data contract requests.
    CreateDataContractRequest,
    /// A data trigger that handles the creation of domain documents.
    DataTriggerCreateDomain,
    /// A data trigger that handles the creation of masternode reward share documents.
    DataTriggerRewardShare,
    /// A data trigger that handles the rejection of documents.
    DataTriggerReject,
    /// A data trigger that handles the creation of feature flag documents.
    #[default]
    CrateFeatureFlag,
    /// A data trigger that handles the deletion of withdrawal documents.
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

/// A struct representing a data trigger on the blockchain.
///
/// The `DataTrigger` struct contains information about a data trigger, including the data contract ID, the document
/// type that the trigger handles, the kind of trigger, the action that triggered the trigger, and an optional
/// identifier for the top-level identity associated with the document.
#[derive(Default, Clone)]
pub struct DataTrigger {
    /// The identifier of the data contract associated with the trigger.
    pub data_contract_id: Identifier,
    /// The type of document that the trigger handles.
    pub document_type: String,
    /// The kind of data trigger.
    pub data_trigger_kind: DataTriggerKind,
    /// The action that triggered the trigger.
    pub transition_action: Action,
}

impl DataTrigger {
    /// Checks whether the data trigger matches the specified data contract ID, document type, and action.
    ///
    /// This function compares the fields of the `DataTrigger` struct with the specified data contract ID, document type,
    /// and action to determine whether the trigger matches. It returns `true` if the trigger matches and `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - A reference to the data contract ID to match.
    /// * `document_type` - A reference to the document type to match.
    /// * `transition_action` - The action to match.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the trigger matches the specified data contract ID, document type, and action.
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

    /// Executes the data trigger using the specified document transition and execution context.
    ///
    /// This function executes the data trigger using the specified `DocumentTransitionAction` and
    /// `DataTriggerExecutionContext`. It calls the `execute_trigger` function to perform the trigger
    /// execution, passing in the trigger kind, document transition, execution context, and top-level
    /// identity. It then returns a `DataTriggerExecutionResult` containing either a successful result or
    /// a `DataTriggerActionError`, indicating the failure of the trigger.
    ///
    /// # Arguments
    ///
    /// * `document_transition` - A reference to the document transition that triggered the data trigger.
    /// * `context` - A reference to the data trigger execution context.
    ///
    /// # Returns
    ///
    /// A `DataTriggerExecutionResult` containing either a successful result or a `DataTriggerActionError`,
    /// indicating the failure of the trigger.
    pub fn execute(
        &self,
        document_transition: &DocumentTransitionAction,
        context: &DataTriggerExecutionContext<'_>,
    ) -> DataTriggerExecutionResult {
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
                let consensus_error = DataTriggerActionError::DataTriggerExecutionError {
                    data_contract_id,
                    document_transition_id: *get_from_transition_action!(document_transition, id),
                    message: err.to_string(),
                    execution_error: err.to_string(),
                    document_transition: None,
                    owner_id: None,
                };
                result.add_error(consensus_error);
                result
            }

            Ok(execution_result) => execution_result,
        }
    }
}

fn execute_trigger(
    trigger_kind: DataTriggerKind,
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    identifier: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, Error> {
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
    dt_create: &DocumentCreateTransitionAction,
    msg: String,
) -> DataTriggerActionError {
    DataTriggerActionError::DataTriggerConditionError {
        data_contract_id: context.data_contract.id,
        document_transition_id: dt_create.base.id,
        message: msg,
        owner_id: Some(*context.owner_id),
        document_transition: Some(DocumentTransitionAction::CreateAction(dt_create.clone())),
    }
}
