use crate::execution::validation::data_trigger::get_data_triggers_factory::{
    data_triggers, get_data_triggers,
};
use crate::execution::validation::data_trigger::{
    DataTrigger, DataTriggerExecutionContext, DataTriggerExecutionResult,
};

use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransitionAction;
use dpp::ProtocolError;
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;

pub(crate) fn execute_data_triggers<'a>(
    document_transitions: &'a [DocumentTransitionAction],
    context: &DataTriggerExecutionContext<'a>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError> {
    let data_triggers_list = data_triggers()?;
    execute_data_triggers_with_custom_list(document_transitions, context, data_triggers_list)
}

pub(crate) fn execute_data_triggers_with_custom_list<'a>(
    document_transitions: &'a [DocumentTransitionAction],
    context: &DataTriggerExecutionContext<'a>,
    data_triggers_list: impl IntoIterator<Item = DataTrigger>,
) -> Result<Vec<DataTriggerExecutionResult>, ProtocolError> {
    let data_contract_id = &context.data_contract.id;
    let mut execution_results: Vec<DataTriggerExecutionResult> = vec![];
    let data_triggers: Vec<DataTrigger> = data_triggers_list.into_iter().collect();

    for document_transition in document_transitions {
        let document_type_name = &document_transition.base().document_type_name;
        let transition_action = document_transition.action();

        let data_triggers_for_transition = get_data_triggers(
            data_contract_id,
            document_type_name,
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
    document_transition: &'a DocumentTransitionAction,
    data_triggers: &[&DataTrigger],
    context: &DataTriggerExecutionContext<'a>,
    results: &mut Vec<DataTriggerExecutionResult>,
) {
    results.extend(
        data_triggers
            .iter()
            .map(|data_trigger| data_trigger.execute(document_transition, context)),
    );
}
