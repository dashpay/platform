use dpp::document::validation::state::execute_data_triggers::execute_data_triggers_with_custom_list;
use dpp::prelude::Identifier;
use dpp::prelude::{DataTrigger, DocumentTransition};
use js_sys::Array;
use wasm_bindgen::prelude::*;

use crate::{
    document_batch_transition::document_transition::DocumentTransitionWasm,
    state_repository::ExternalStateRepositoryLikeWrapper,
    utils::{Inner, IntoWasm, WithJsError},
    DataTriggerWasm,
};

use super::{
    data_trigger_execution_context::{
        data_trigger_context_from_wasm, DataTriggerExecutionContextWasm,
    },
    data_trigger_execution_result::DataTriggerExecutionResultWasm,
};

#[wasm_bindgen(js_name=executeDataTriggers)]
pub async fn execute_data_triggers_wasm(
    js_document_transitions: Array,
    js_context: DataTriggerExecutionContextWasm,
    js_data_triggers: Array,
) -> Result<Array, JsValue> {
    let st_wrapper =
        ExternalStateRepositoryLikeWrapper::new_with_arc(js_context.state_repository());
    let identifier: Identifier = js_context.get_owner_id().into();
    let context = data_trigger_context_from_wasm(&js_context, &st_wrapper, &identifier);
    let document_transitions = js_document_transitions
        .iter()
        .map(|v| {
            v.to_wasm::<DocumentTransitionWasm>("DocumentTransition")
                .map(|v| v.to_owned().into_inner())
        })
        .collect::<Result<Vec<DocumentTransition>, JsValue>>()?;

    let data_triggers = js_data_triggers
        .iter()
        .map(|v| {
            v.to_wasm::<DataTriggerWasm>("DataTrigger")
                .map(|v| v.to_owned().into_inner())
        })
        .collect::<Result<Vec<DataTrigger>, JsValue>>()?;

    let results = execute_data_triggers_with_custom_list(
        document_transitions.iter().collect::<Vec<_>>().as_slice(),
        &context,
        data_triggers,
    )
    .await
    .with_js_error()?;

    let array_with_results = js_sys::Array::new();
    for result in results {
        let result_wasm: DataTriggerExecutionResultWasm = result.into();
        array_with_results.push(&result_wasm.into());
    }

    Ok(array_with_results)
}
