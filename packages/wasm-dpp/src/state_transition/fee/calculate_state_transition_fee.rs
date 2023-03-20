use dpp::{
    consensus::basic::state_transition,
    state_transition::{
        fee::calculate_state_transition_fee::calculate_state_transition_fee, StateTransition,
    },
};
use js_sys::BigInt;
use wasm_bindgen::prelude::*;

use crate::conversion::create_state_transition_from_wasm_instance;

#[wasm_bindgen(js_name=calculateStateTransitionFee)]
pub fn calculate_state_transition_fee_wasm(
    state_transition_js: &JsValue,
) -> Result<BigInt, JsValue> {
    let state_transition = create_state_transition_from_wasm_instance(state_transition_js)?;

    let fee = calculate_state_transition_fee(&state_transition);
    Ok(BigInt::from(fee))
}
