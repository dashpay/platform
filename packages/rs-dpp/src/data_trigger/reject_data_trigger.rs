use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use crate::{
    document::document_transition::DocumentTransition, get_from_transition, prelude::Identifier,
    state_repository::StateRepositoryLike,
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

pub async fn reject_data_trigger<'a, SR>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<'a, SR>,
    _top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
{
    let mut result = DataTriggerExecutionResult::default();

    result.add_error(
        DataTriggerError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id,
            document_transition_id: get_from_transition!(document_transition, id).to_owned(),
            message: String::from("Action is not allowed"),
            document_transition: None,
            owner_id: None,
        }
        .into(),
    );

    Ok(result)
}
