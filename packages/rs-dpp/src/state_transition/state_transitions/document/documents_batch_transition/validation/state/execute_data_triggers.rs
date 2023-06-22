use crate::{
    data_trigger::{
        get_data_triggers_factory::{data_triggers, get_data_triggers},
        DataTrigger, DataTriggerExecutionContext, DataTriggerExecutionResult,
    },
    document::document_transition::DocumentTransition,
    state_repository::StateRepositoryLike,
    ProtocolError,
};

pub async fn execute_data_triggers<'a, SR>(
    document_transitions: &[&DocumentTransition],
    context: &DataTriggerExecutionContext<'a, SR>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError>
where
    SR: StateRepositoryLike,
{
    let data_triggers_list = data_triggers()?;
    execute_data_triggers_with_custom_list(document_transitions, context, data_triggers_list).await
}

pub async fn execute_data_triggers_with_custom_list<'a, SR>(
    document_transitions: &[&DocumentTransition],
    context: &DataTriggerExecutionContext<'a, SR>,
    data_triggers_list: impl IntoIterator<Item = DataTrigger>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError>
where
    SR: StateRepositoryLike,
{
    let data_contract_id = &context.data_contract.id;
    let mut execution_results: Vec<DataTriggerExecutionResult> = vec![];
    let data_triggers: Vec<DataTrigger> = data_triggers_list.into_iter().collect();

    for dt in document_transitions {
        let document_transition = dt;
        let document_type = &document_transition.base().document_type_name;
        let transition_action = document_transition.base().action;

        let data_triggers_for_transition = get_data_triggers(
            data_contract_id,
            document_type,
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
        )
        .await;
    }

    Ok(execution_results)
}

async fn execute_data_triggers_sequentially<'a, SR>(
    document_transition: &'a DocumentTransition,
    data_triggers: &[&DataTrigger],
    context: &DataTriggerExecutionContext<'a, SR>,
    results: &'a mut Vec<DataTriggerExecutionResult>,
) where
    SR: StateRepositoryLike,
{
    for data_trigger in data_triggers {
        results.push(data_trigger.execute(document_transition, context).await);
    }
}
