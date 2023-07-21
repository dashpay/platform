///! Data triggers implement custom validation logic for state transitions
///! that modifies documents in a specific data contract.
///! Data triggers can be assigned based on the data contract ID, document type, and action.
use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;
use dpp::document::document_transition::{
    Action, DocumentCreateTransitionAction, DocumentTransitionAction,
};
use dpp::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::validation::SimpleValidationResult;

use crate::error::Error;
use dpp::version::PlatformVersion;

pub(super) use bindings::list::data_trigger_bindings_list;
pub(super) use context::DataTriggerExecutionContext;
pub(super) use executor::DataTriggerExecutor;

mod bindings;
mod context;
mod executor;
mod triggers;

type DataTrigger = Box<
    dyn Fn(
        &DocumentTransitionAction,
        &DataTriggerExecutionContext<'_>,
        &PlatformVersion,
    ) -> Result<DataTriggerExecutionResult, Error>,
>;

/// A type alias for a `SimpleValidationResult` with a `DataTriggerActionError` as the error type.
///
/// This type is used to represent the result of executing a data trigger on the blockchain. It contains either a
/// successful result or a `DataTriggerActionError`, indicating the failure of the trigger.
type DataTriggerExecutionResult = SimpleValidationResult<DataTriggerActionError>;

/// An enumeration representing the different kinds of data triggers that can be executed on the blockchain.
///
/// Each variant of the enum corresponds to a specific type of data trigger that can be executed. The enum is used
/// throughout the data trigger system to identify the type of trigger that is being executed.
// #[derive(Debug, Clone, Copy, Default)]
// pub enum DataTrigger {
//     /// A data trigger that handles the creation of data contract requests.
//     CreateDataContractRequest(create_contact_request_data_trigger),
//     /// A data trigger that handles the creation of domain documents.
//     DataTriggerCreateDomain(create_domain_data_trigger),
//     /// A data trigger that handles the creation of masternode reward share documents.
//     DataTriggerRewardShare(create_feature_flag_data_trigger),
//     /// A data trigger that handles the rejection of documents.
//     DataTriggerReject(reject_data_trigger),
//     /// A data trigger that handles the creation of feature flag documents.
//     CrateFeatureFlag(create_feature_flag_data_trigger),
//     /// A data trigger that handles the deletion of withdrawal documents.
//     DeleteWithdrawal(delete_withdrawal_data_trigger),
// }
//
// impl From<DataTrigger> for &str {
//     fn from(value: DataTrigger) -> Self {
//         match value {
//             DataTrigger::CrateFeatureFlag(_) => "createFeatureFlag",
//             DataTrigger::DataTriggerReject(_) => "dataTriggerReject",
//             DataTrigger::DataTriggerRewardShare(_) => "dataTriggerRewardShare",
//             DataTrigger::DataTriggerCreateDomain(_) => "dataTriggerCreateDomain",
//             DataTrigger::CreateDataContractRequest(_) => "createDataContractRequest",
//             DataTrigger::DeleteWithdrawal(_) => "deleteWithdrawal",
//         }
//     }
// }

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
