use crate::{
    document::document_transition::DocumentTransition,
    errors::DataTriggerError,
    get_from_transition,
    state_repository::{SMLStoreLike, SimplifiedMNListLike, StateRepositoryLike},
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

pub async fn reject_data_trigger<SR, S, L>(
    document_transition: &DocumentTransition,
    context: DataTriggerExecutionContext<SR, S, L>,
) where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
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
}
