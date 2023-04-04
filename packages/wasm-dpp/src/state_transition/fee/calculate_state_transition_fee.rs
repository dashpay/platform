use dpp::{
    state_transition::fee::calculate_state_transition_fee_factory::calculate_state_transition_fee,
    ProtocolError,
};
use wasm_bindgen::prelude::*;

use crate::{
    conversion::create_state_transition_from_wasm_instance, fee::fee_result::FeeResultWasm,
    utils::WithJsError, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=calculateStateTransitionFee)]
pub fn calculate_state_transition_fee_wasm(
    state_transition_js: &JsValue,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<FeeResultWasm, JsValue> {
    let state_transition = create_state_transition_from_wasm_instance(state_transition_js)?;

    Ok(
        calculate_state_transition_fee(&state_transition, &execution_context.to_owned().into())
            .map_err(ProtocolError::from)
            .with_js_error()?
            .into(),
    )
}
