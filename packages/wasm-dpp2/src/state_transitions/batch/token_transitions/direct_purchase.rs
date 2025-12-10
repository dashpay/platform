use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::TokenDirectPurchaseTransitionV0;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenDirectPurchaseTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenDirectPurchaseTransition)]
pub struct TokenDirectPurchaseTransitionWasm(TokenDirectPurchaseTransition);

impl From<TokenDirectPurchaseTransitionWasm> for TokenDirectPurchaseTransition {
    fn from(transition: TokenDirectPurchaseTransitionWasm) -> Self {
        transition.0
    }
}

impl From<TokenDirectPurchaseTransition> for TokenDirectPurchaseTransitionWasm {
    fn from(transition: TokenDirectPurchaseTransition) -> Self {
        TokenDirectPurchaseTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = TokenDirectPurchaseTransition)]
impl TokenDirectPurchaseTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenDirectPurchaseTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenDirectPurchaseTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWasm,
        token_count: TokenAmount,
        total_agreed_price: Credits,
    ) -> Self {
        TokenDirectPurchaseTransitionWasm(TokenDirectPurchaseTransition::V0(
            TokenDirectPurchaseTransitionV0 {
                base: base.clone().into(),
                token_count,
                total_agreed_price,
            },
        ))
    }

    #[wasm_bindgen(getter = base)]
    pub fn get_base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = tokenCount)]
    pub fn get_token_count(&self) -> TokenAmount {
        self.0.token_count()
    }

    #[wasm_bindgen(getter = totalAgreedPrice)]
    pub fn get_total_agreed_price(&self) -> Credits {
        self.0.total_agreed_price()
    }

    #[wasm_bindgen(setter = base)]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = tokenCount)]
    pub fn set_token_count(&mut self, token_count: TokenAmount) {
        self.0.set_token_count(token_count)
    }

    #[wasm_bindgen(setter = totalAgreedPrice)]
    pub fn set_total_agreed_price(&mut self, total_agreed_price: Credits) {
        self.0.set_total_agreed_price(total_agreed_price)
    }
}
