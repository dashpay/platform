use wasm_bindgen::prelude::wasm_bindgen;
use dpp::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::transition::TokenOrderSellLimitTransition;

#[wasm_bindgen(js_name=TokenOrderSellLimitTransition)]
#[derive(Debug, Clone)]
pub struct TokenOrderSellLimitTransitionWasm(TokenOrderSellLimitTransition);

impl From<TokenOrderSellLimitTransition> for TokenOrderSellLimitTransitionWasm {
    fn from(value: TokenOrderSellLimitTransition) -> Self {
        Self(value)
    }
}
