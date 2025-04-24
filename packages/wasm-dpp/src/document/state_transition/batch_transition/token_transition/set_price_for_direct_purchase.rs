use dpp::state_transition::batch_transition::TokenSetPriceForDirectPurchaseTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenSetPriceForDirectPurchaseTransition)]
#[derive(Debug, Clone)]
pub struct TokenSetPriceForDirectPurchaseTransitionWasm(TokenSetPriceForDirectPurchaseTransition);

impl From<TokenSetPriceForDirectPurchaseTransition>
    for TokenSetPriceForDirectPurchaseTransitionWasm
{
    fn from(value: TokenSetPriceForDirectPurchaseTransition) -> Self {
        Self(value)
    }
}
