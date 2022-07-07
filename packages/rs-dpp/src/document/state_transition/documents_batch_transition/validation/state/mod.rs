use std::borrow::Borrow;

use crate::{
    data_trigger::{
        self, get_data_triggers_factory::get_data_triggers, DataTrigger,
        DataTriggerExecutionContext, DataTriggerExecutionResult,
    },
    document::document_transition::DocumentTransition,
    state_repository::StateRepositoryLike,
    ProtocolError,
};

pub mod execute_data_triggers;
pub mod fetch_documents_factory;
pub mod validate_documents_batch_transition_state;
pub mod validate_documents_uniqueness_by_indices;

async fn execute_data_triggers<SR>(
    document_transitions: impl IntoIterator<Item = impl Borrow<DocumentTransition>>,
    context: DataTriggerExecutionContext<SR>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError>
where
    SR: StateRepositoryLike,
{
    let data_contract_id = &context.data_contract.id;
    let mut execution_results: Vec<DataTriggerExecutionResult> = vec![];

    for document_transition in document_transitions {
        let dt = document_transition.borrow();
        let document_type = &dt.base().document_type;
        let transition_action = dt.base().action;

        let data_triggers_for_transition =
            get_data_triggers::<SR>(data_contract_id, document_type, transition_action)?;

        if data_triggers_for_transition.is_empty() {
            continue;
        }

        execute_data_triggers_sequentially(
            dt,
            data_triggers_for_transition,
            &context,
            &mut execution_results,
        )
        .await;
    }

    Ok(execution_results)
}

async fn execute_data_triggers_sequentially<'a, 'b, SR>(
    document_transition: &'a DocumentTransition,
    data_triggers: Vec<DataTrigger>,
    context: &'a DataTriggerExecutionContext<SR>,
    results: &'a mut Vec<DataTriggerExecutionResult>,
) where
    'b: 'a,
    SR: StateRepositoryLike,
{
    for data_trigger in data_triggers.into_iter() {
        results.push(data_trigger.execute(document_transition, context).await);
    }
}
