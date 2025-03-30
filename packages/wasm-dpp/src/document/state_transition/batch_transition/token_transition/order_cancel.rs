use wasm_bindgen::prelude::wasm_bindgen;
use dpp::state_transition::batch_transition::batched_transition::token_order_cancel_transition::transition::TokenOrderCancelTransition;

#[wasm_bindgen(js_name=TokenOrderCancelTransition)]
#[derive(Debug, Clone)]
pub struct TokenOrderCancelTransitionWasm(TokenOrderCancelTransition);

impl From<TokenOrderCancelTransition> for TokenOrderCancelTransitionWasm {
    fn from(value: TokenOrderCancelTransition) -> Self {
        Self(value)
    }
}
