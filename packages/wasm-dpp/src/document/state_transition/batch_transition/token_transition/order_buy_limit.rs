use wasm_bindgen::prelude::wasm_bindgen;
use dpp::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::transition::TokenOrderBuyLimitTransition;

#[wasm_bindgen(js_name=TokenOrderBuyLimitTransition)]
#[derive(Debug, Clone)]
pub struct TokenOrderBuyLimitTransitionWasm(TokenOrderBuyLimitTransition);

impl From<TokenOrderBuyLimitTransition> for TokenOrderBuyLimitTransitionWasm {
    fn from(value: TokenOrderBuyLimitTransition) -> Self {
        Self(value)
    }
}
