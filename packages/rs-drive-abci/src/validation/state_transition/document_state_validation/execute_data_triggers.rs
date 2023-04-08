use dpp::prelude::DocumentTransition;
use dpp::ProtocolError;
use drive::drive::Drive;
use crate::execution::data_trigger::{DataTrigger, DataTriggerExecutionContext, DataTriggerExecutionResult};
use crate::execution::data_trigger::get_data_triggers_factory::{data_triggers, get_data_triggers};

pub fn execute_data_triggers<'a>(
    document_transitions: &[&DocumentTransition],
    context: &DataTriggerExecutionContext<'a>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError>
{
    let data_triggers_list = data_triggers()?;
    execute_data_triggers_with_custom_list(document_transitions, context, data_triggers_list)
}

pub fn execute_data_triggers_with_custom_list<'a>(
    document_transitions: &[&DocumentTransition],
    context: &DataTriggerExecutionContext<'a>,
    data_triggers_list: impl IntoIterator<Item = DataTrigger>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError>
{
    let data_contract_id = &context.data_contract.id;
    let mut execution_results: Vec<DataTriggerExecutionResult> = vec![];
    let data_triggers: Vec<DataTrigger> = data_triggers_list.into_iter().collect();

    for document_transition in document_transitions {
        let document_transition = document_transition.as_ref();
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
        );
    }

    Ok(execution_results)
}

fn execute_data_triggers_sequentially<'a>(
    document_transition: &'a DocumentTransition,
    data_triggers: &[&DataTrigger],
    context: &DataTriggerExecutionContext<'a>,
    results: &'a mut Vec<DataTriggerExecutionResult>,
)
{
    results.extend(data_triggers.iter().map(|data_trigger| data_trigger.execute(document_transition, context)));
}
