use crate::error::Error;
use dpp::document::document_transition::DocumentTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::{
    document::document_transition::DocumentTransition, errors::DataTriggerError,
    get_from_transition_action, prelude::Identifier, DataTriggerActionError,
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

pub fn reject_data_trigger<'a>(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'a>,
    _top_level_identity: Option<&Identifier>,
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
