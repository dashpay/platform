use dpp::state_transition::batch_transition::TokenConfigUpdateTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenConfigUpdateTransition)]
#[derive(Debug, Clone)]
pub struct TokenConfigUpdateTransitionWasm(TokenConfigUpdateTransition);

impl From<TokenConfigUpdateTransition> for TokenConfigUpdateTransitionWasm {
    fn from(value: TokenConfigUpdateTransition) -> Self {
        Self(value)
    }
}
