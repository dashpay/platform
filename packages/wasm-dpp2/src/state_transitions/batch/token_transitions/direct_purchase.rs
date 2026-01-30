use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_with, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::TokenDirectPurchaseTransitionV0;
use dpp::state_transition::batch_transition::TokenDirectPurchaseTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_DIRECT_PURCHASE_OPTIONS_TS: &str = r#"
export interface TokenDirectPurchaseTransitionOptions {
    base: TokenBaseTransition;
    tokenCount: bigint;
    totalAgreedPrice: bigint;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenDirectPurchaseTransitionOptions")]
    pub type TokenDirectPurchaseTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenDirectPurchaseTransition")]
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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenDirectPurchaseTransitionOptionsJs,
    ) -> WasmDppResult<TokenDirectPurchaseTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let token_count: TokenAmount =
            try_from_options_with(&options, "tokenCount", |v| try_to_u64(v, "tokenCount"))?;

        let total_agreed_price: Credits = try_from_options_with(&options, "totalAgreedPrice", |v| {
            try_to_u64(v, "totalAgreedPrice")
        })?;

        Ok(TokenDirectPurchaseTransitionWasm(
            TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0 {
                base: base.into(),
                token_count,
                total_agreed_price,
            }),
        ))
    }

    #[wasm_bindgen(getter = base)]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = tokenCount)]
    pub fn token_count(&self) -> TokenAmount {
        self.0.token_count()
    }

    #[wasm_bindgen(getter = totalAgreedPrice)]
    pub fn total_agreed_price(&self) -> Credits {
        self.0.total_agreed_price()
    }

    #[wasm_bindgen(setter = base)]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = tokenCount)]
    pub fn set_token_count(
        &mut self,
        #[wasm_bindgen(js_name = "tokenCount")] token_count: TokenAmount,
    ) {
        self.0.set_token_count(token_count)
    }

    #[wasm_bindgen(setter = totalAgreedPrice)]
    pub fn set_total_agreed_price(
        &mut self,
        #[wasm_bindgen(js_name = "totalAgreedPrice")] total_agreed_price: Credits,
    ) {
        self.0.set_total_agreed_price(total_agreed_price)
    }
}

impl_try_from_js_value!(TokenDirectPurchaseTransitionWasm, "TokenDirectPurchaseTransition");
impl_wasm_type_info!(
    TokenDirectPurchaseTransitionWasm,
    TokenDirectPurchaseTransition
);
