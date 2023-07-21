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

pub use context::DataTriggerExecutionContext;
use dpp::ProtocolError;

mod bindings;
mod context;
mod triggers;

/// A type alias for a `SimpleValidationResult` with a `DataTriggerActionError` as the error type.
///
/// This type is used to represent the result of executing a data trigger on the blockchain. It contains either a
/// successful result or a `DataTriggerActionError`, indicating the failure of the trigger.
pub type DataTriggerExecutionResult = SimpleValidationResult<DataTriggerActionError>;

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

pub(crate) fn execute_data_triggers<'a>(
    document_transitions: &'a [DocumentTransitionAction],
    context: &DataTriggerExecutionContext<'a>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError> {
    let data_triggers_list = data_triggers()?;
    execute_data_triggers_with_custom_list(document_transitions, context, data_triggers_list)
}

pub(crate) fn execute_data_triggers_with_custom_list<'a>(
    document_transitions: &'a [DocumentTransitionAction],
    context: &DataTriggerExecutionContext<'a>,
    data_triggers_list: impl IntoIterator<Item = DataTriggerBinding>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError> {
    let data_contract_id = &context.data_contract.id;
    let mut execution_results: Vec<DataTriggerExecutionResult> = vec![];
    let data_triggers: Vec<DataTriggerBinding> = data_triggers_list.into_iter().collect();

    for document_transition in document_transitions {
        let document_type_name = &document_transition.base().document_type_name;
        let transition_action = document_transition.action();

        let data_triggers_for_transition = get_data_triggers(
            data_contract_id,
            document_type_name,
            transition_action,
            data_triggers.iter(),
        )?;

        if data_triggers_for_transition.is_empty() {
            continue;
        }

        execute_data_triggers_sequentially(
            document_transition,
            &data_triggers_for_transition,
            context,
            &mut execution_results,
        );
    }

    Ok(execution_results)
}

fn execute_data_triggers_sequentially<'a>(
    document_transition: &'a DocumentTransitionAction,
    data_triggers: &[&DataTriggerBinding],
    context: &DataTriggerExecutionContext<'a>,
    results: &mut Vec<DataTriggerExecutionResult>,
) {
    results.extend(
        data_triggers
            .iter()
            .map(|data_trigger| data_trigger.execute(document_transition, context)),
    );
}

/// returns Date Triggers filtered out by dataContractId, documentType, transactionAction
pub fn get_data_triggers<'a>(
    data_contract_id: &'a Identifier,
    document_type_name: &'a str,
    transition_action: Action,
    data_triggers_list: impl IntoIterator<Item = &'a DataTriggerBinding>,
) -> Result<Vec<&'a DataTriggerBinding>, ProtocolError> {
    Ok(data_triggers_list
        .into_iter()
        .filter(|dt| {
            dt.is_matching_trigger_for_data(data_contract_id, document_type_name, transition_action)
        })
        .collect())
}
