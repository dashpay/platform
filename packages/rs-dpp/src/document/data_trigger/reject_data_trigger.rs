use crate::{
    document::document_transition::DocumentTransition, errors::StateError,
    state_repository::StateRepositoryLike,
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

async fn reject_data_trigger<SR>(
    document_transition: DocumentTransition,
    context: DataTriggerExecutionContext<SR>,
) where
    SR: StateRepositoryLike,
{
    let mut result = DataTriggerExecutionResult::default();
    // result.add_error(StateEr)
}
