use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::validation::state_transition::documents_batch::data_triggers::DataTriggerExecutionResult;
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use crate::error::execution::ExecutionError;

/// Creates a data trigger for handling document rejections.
///
/// The trigger is executed whenever a document is rejected on the blockchain.
/// It performs various actions depending on the state of the document and the context in which it was rejected.
///
/// # Arguments
///
/// * `document_transition` - A reference to the document transition that triggered the data trigger.
/// * `context` - A reference to the data trigger execution context.
/// * `_top_level_identity` - An unused parameter for the top-level identity associated with the rejected document
///   (which is not needed for this trigger).
///
/// # Returns
///
/// A `SimpleValidationResult` containing either a `DataTriggerActionError` indicating the failure of the trigger
/// or an empty result indicating the success of the trigger.
#[inline(always)]
pub(super) fn reject_data_trigger_v0(
    document_transition: &DocumentTransitionAction,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_contract_fetch_info = document_transition
        .base()
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "expecting action to have a base",
        )))?
        .data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;
    let mut result = DataTriggerExecutionResult::default();

    let err = DataTriggerConditionError::new(
        data_contract.id(),
        document_transition
            .base()
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "expecting action to have a base",
            )))?
            .id(),
        "Action is not allowed".to_string(),
    );

    result.add_error(err);

    Ok(result)
}
