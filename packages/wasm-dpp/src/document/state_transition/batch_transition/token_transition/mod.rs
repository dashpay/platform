use wasm_bindgen::prelude::wasm_bindgen;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;

#[wasm_bindgen(js_name=TokenTransition)]
#[derive(Debug, Clone)]
pub struct TokenTransitionWasm(TokenTransition);

impl From<TokenTransition> for TokenTransitionWasm {
    fn from(t: TokenTransition) -> Self {
        TokenTransitionWasm(t)
    }
}

impl From<TokenTransitionWasm> for TokenTransition {
    fn from(t: TokenTransitionWasm) -> Self {
        t.0
    }
}
