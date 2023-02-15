use dpp::{
    data_trigger::get_data_triggers_factory::get_data_triggers,
    document::validation::state::execute_data_triggers::execute_data_triggers,
};
use itertools::Itertools;
use js_sys::Array;
use wasm_bindgen::prelude::*;

use crate::{
    console_log,
    document_batch_transition::document_transition::DocumentTransitionWasm,
    state_repository::ExternalStateRepositoryLikeWrapper,
    utils::{Inner, IntoWasm, WithJsError},
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
) -> Result<Array, JsValue> {
    console_error_panic_hook::set_once();
    let st_wrapper =
        ExternalStateRepositoryLikeWrapper::new_with_arc(js_context.state_repository());
    let context = data_trigger_context_from_wasm(&js_context, &st_wrapper);
    let mut document_transitions: Vec<_> = js_document_transitions
        .iter()
        .map(|v| {
            v.to_wasm::<DocumentTransitionWasm>("DocumentTransition")
                .map(|v| v.to_owned())
        })
        .try_collect()?;

    // let document_transition = document_transitions
    //     .pop()
    //     .expect("at least one transition should be given");

    // let mut data_triggers = get_data_triggers(
    //     &context.data_contract.id,
    //     &document_transition.inner().base().document_type,
    //     document_transition.inner().base().action,
    // )
    // .expect("no errors durring selection data trigger should happen");

    // console_log!(
    //     "selected {} data triggers for transition",
    //     data_triggers.len()
    // );

    // let data_trigger = data_triggers.pop().expect("");
    // let result = data_trigger
    //     .execute(document_transition.inner(), &context)
    //     .await;
    // console_log!("the execution result is {} len", result.get_errors().len());

    // // Ok(js_sys::Array::new())
    console_log!("trying to execute data triggers from rs-dpp");
    let results = execute_data_triggers(document_transitions.iter().map(|v| v.inner()), &context)
        .await
        .with_js_error()?;

    console_log!("converting results into array");
    let array_with_results = js_sys::Array::new();
    for result in results {
        let result_wasm: DataTriggerExecutionResultWasm = result.into();
        array_with_results.push(&result_wasm.into());
    }

    console_log!("converting results into array");
    Ok(array_with_results)
}
