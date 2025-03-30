use wasm_bindgen::prelude::wasm_bindgen;
use dpp::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::transition::TokenOrderAdjustPriceTransition;

#[wasm_bindgen(js_name=TokenOrderAdjustPriceTransition)]
#[derive(Debug, Clone)]
pub struct TokenOrderAdjustPriceTransitionWasm(TokenOrderAdjustPriceTransition);

impl From<TokenOrderAdjustPriceTransition> for TokenOrderAdjustPriceTransitionWasm {
    fn from(value: TokenOrderAdjustPriceTransition) -> Self {
        Self(value)
    }
}
