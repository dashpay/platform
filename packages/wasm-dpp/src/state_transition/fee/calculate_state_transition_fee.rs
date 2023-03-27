use dpp::state_transition::fee::calculate_state_transition_fee_factory::calculate_state_transition_fee;
use wasm_bindgen::prelude::*;

use crate::{
    conversion::create_state_transition_from_wasm_instance, fee::fee_result::FeeResultWasm,
};

#[wasm_bindgen(js_name=calculateStateTransitionFee)]
pub fn calculate_state_transition_fee_wasm(
    state_transition_js: &JsValue,
) -> Result<FeeResultWasm, JsValue> {
    let state_transition = create_state_transition_from_wasm_instance(state_transition_js)?;

    Ok(calculate_state_transition_fee(&state_transition).into())
}
