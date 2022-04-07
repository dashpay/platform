use crate::{
    document::document_transition::DocumentTransition, errors::DataTriggerError,
    get_from_transition, state_repository::StateRepositoryLike,
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

async fn reject_data_trigger<SR>(
    document_transition: DocumentTransition,
    context: DataTriggerExecutionContext<SR>,
) where
    SR: StateRepositoryLike,
{
    let mut result = DataTriggerExecutionResult::default();

    result.add_error(
        DataTriggerError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id.clone(),
            document_transition_id: get_from_transition!(document_transition, id).clone(),
            message: String::from("Action is not allowed"),
        }
        .into(),
    );
}
