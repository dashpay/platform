use crate::{
    data_trigger::{
        get_data_triggers_factory::get_data_triggers, DataTrigger, DataTriggerExecutionContext,
        DataTriggerExecutionResult,
    },
    document::document_transition::DocumentTransition,
    state_repository::StateRepositoryLike,
    ProtocolError,
};

pub async fn execute_data_triggers<'a, SR>(
    document_transitions: impl IntoIterator<Item = impl AsRef<DocumentTransition>>,
    context: &DataTriggerExecutionContext<'a, SR>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError>
where
    SR: StateRepositoryLike,
{
    let data_contract_id = &context.data_contract.id;
    let mut execution_results: Vec<DataTriggerExecutionResult> = vec![];

    for dt in document_transitions {
        let document_transition = dt.as_ref();
        let document_type = &document_transition.base().document_type;
        let transition_action = document_transition.base().action;

        let data_triggers_for_transition =
            get_data_triggers::<SR>(data_contract_id, document_type, transition_action)?;

        if data_triggers_for_transition.is_empty() {
            continue;
        }

        execute_data_triggers_sequentially(
            document_transition,
            data_triggers_for_transition,
            context,
            &mut execution_results,
        )
        .await;
    }

    Ok(execution_results)
}

async fn execute_data_triggers_sequentially<'a, SR>(
    document_transition: &'a DocumentTransition,
    data_triggers: Vec<DataTrigger>,
    context: &DataTriggerExecutionContext<'a, SR>,
    results: &'a mut Vec<DataTriggerExecutionResult>,
) where
    SR: StateRepositoryLike,
{
    for data_trigger in data_triggers.into_iter() {
        results.push(data_trigger.execute(document_transition, context).await);
    }
}
