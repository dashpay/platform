use dpp::state_transition::batch_transition::TokenClaimTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenClaimTransition)]
#[derive(Debug, Clone)]
pub struct TokenClaimTransitionWasm(TokenClaimTransition);

impl From<TokenClaimTransition> for TokenClaimTransitionWasm {
    fn from(value: TokenClaimTransition) -> Self {
        Self(value)
    }
}
