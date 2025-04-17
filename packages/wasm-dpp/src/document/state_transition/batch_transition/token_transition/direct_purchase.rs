use dpp::state_transition::batch_transition::TokenDirectPurchaseTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenDirectPurchaseTransition)]
#[derive(Debug, Clone)]
pub struct TokenDirectPurchaseTransitionWasm(TokenDirectPurchaseTransition);

impl From<TokenDirectPurchaseTransition> for TokenDirectPurchaseTransitionWasm {
    fn from(value: TokenDirectPurchaseTransition) -> Self {
        Self(value)
    }
}
