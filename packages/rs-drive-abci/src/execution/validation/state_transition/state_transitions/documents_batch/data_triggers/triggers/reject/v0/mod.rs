use crate::error::Error;
use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;

use crate::execution::validation::data_trigger::DataTriggerExecutionContext;
use crate::execution::validation::state_transition::documents_batch::data_triggers::DataTriggerExecutionContext;
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::version::PlatformVersion;
use dpp::{get_from_transition_action, prelude::Identifier};

use super::DataTriggerExecutionContext;

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
pub fn reject_data_trigger_v0(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<SimpleValidationResult<DataTriggerActionError>, Error> {
    let mut result = SimpleValidationResult::<DataTriggerActionError>::default();

    result.add_error(DataTriggerActionError::DataTriggerConditionError {
        data_contract_id: context.data_contract.id,
        document_transition_id: get_from_transition_action!(document_transition, id).to_owned(),
        message: String::from("Action is not allowed"),
        document_transition: None,
        owner_id: None,
    });

    Ok(result)
}
