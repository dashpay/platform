use dpp::state_transition::batch_transition::TokenDirectPurchaseTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

#[wasm_bindgen(js_name=TokenDirectPurchaseTransition)]
#[derive(Debug, Clone)]
pub struct TokenDirectPurchaseTransitionWasm(TokenDirectPurchaseTransition);

impl From<TokenDirectPurchaseTransition> for TokenDirectPurchaseTransitionWasm {
    fn from(value: TokenDirectPurchaseTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenDirectPurchaseTransition)]
impl TokenDirectPurchaseTransitionWasm {
    #[wasm_bindgen(js_name=getCount)]
    pub fn count(&self) -> TokenAmount {
        self.0.token_count()
    }

    #[wasm_bindgen(js_name=getTotalAgreedPrice)]
    pub fn total_agreed_price(&self) -> Credits {
        self.0.total_agreed_price()
    }
}