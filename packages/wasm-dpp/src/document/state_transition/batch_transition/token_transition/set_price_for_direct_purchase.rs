use dpp::state_transition::batch_transition::TokenSetPriceForDirectPurchaseTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

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

#[wasm_bindgen(js_class=TokenSetPriceForDirectPurchaseTransition)]
impl TokenSetPriceForDirectPurchaseTransitionWasm {
    #[wasm_bindgen(js_name=getPublicNote)]
    pub fn public_note(&self) -> Option<String> {
        match self.0.public_note() {
            Some(note) => Some(note.clone()),
            None => None,
        }
    }

    #[wasm_bindgen(js_name=getPrice)]
    pub fn price(&self) -> Option<Credits> {
        match self.0.price() {
            Some(price) => {
                match price {
                    TokenPricingSchedule::SinglePrice(credits) => Some(credits.clone()),
                    TokenPricingSchedule::SetPrices(prices) => None
                }
            },
            None => None
        }
    }
}
