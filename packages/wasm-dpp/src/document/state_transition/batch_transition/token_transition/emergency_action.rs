use dpp::state_transition::state_transitions::document::batch_transition::TokenEmergencyActionTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenEmergencyActionTransition)]
#[derive(Debug, Clone)]
pub struct TokenEmergencyActionTransitionWasm(TokenEmergencyActionTransition);

impl From<TokenEmergencyActionTransition> for TokenEmergencyActionTransitionWasm {
    fn from(value: TokenEmergencyActionTransition) -> Self {
        Self(value)
    }
}
