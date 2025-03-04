use dpp::state_transition::state_transitions::document::batch_transition::TokenBurnTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenBurnTransition)]
#[derive(Debug, Clone)]
pub struct TokenBurnTransitionWasm(TokenBurnTransition);

impl From<TokenBurnTransition> for TokenBurnTransitionWasm {
    fn from(value: TokenBurnTransition) -> Self {
        Self(value)
    }
}
