use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::state_transitions::batch::token_pricing_schedule::TokenPricingScheduleWasm;
use crate::utils::{try_from_options, try_from_options_optional_with, try_to_string, IntoWasm};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransitionV0;
use dpp::state_transition::batch_transition::TokenSetPriceForDirectPurchaseTransition;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use js_sys::Reflect;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_SET_PRICE_OPTIONS_TS: &str = r#"
export interface TokenSetPriceForDirectPurchaseTransitionOptions {
    base: TokenBaseTransition;
    price?: TokenPricingSchedule;
    publicNote?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenSetPriceForDirectPurchaseTransitionOptions")]
    pub type TokenSetPriceForDirectPurchaseTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenSetPriceForDirectPurchaseTransition")]
pub struct TokenSetPriceForDirectPurchaseTransitionWasm(TokenSetPriceForDirectPurchaseTransition);

impl From<TokenSetPriceForDirectPurchaseTransition>
    for TokenSetPriceForDirectPurchaseTransitionWasm
{
    fn from(transition: TokenSetPriceForDirectPurchaseTransition) -> Self {
        TokenSetPriceForDirectPurchaseTransitionWasm(transition)
    }
}

impl From<TokenSetPriceForDirectPurchaseTransitionWasm>
    for TokenSetPriceForDirectPurchaseTransition
{
    fn from(transition: TokenSetPriceForDirectPurchaseTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenSetPriceForDirectPurchaseTransition)]
impl TokenSetPriceForDirectPurchaseTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenSetPriceForDirectPurchaseTransitionOptionsJs,
    ) -> WasmDppResult<TokenSetPriceForDirectPurchaseTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let price: Option<TokenPricingSchedule> =
            Reflect::get(&options, &JsValue::from_str("price"))
                .ok()
                .filter(|v| !v.is_undefined())
                .map(|v| {
                    v.to_wasm::<TokenPricingScheduleWasm>("TokenPricingSchedule")
                        .map(|p| p.clone().into())
                })
                .transpose()?;

        let public_note: Option<String> =
            try_from_options_optional_with(&options, "publicNote", |v| {
                try_to_string(v, "publicNote")
            })?;

        Ok(TokenSetPriceForDirectPurchaseTransitionWasm(
            TokenSetPriceForDirectPurchaseTransition::V0(
                TokenSetPriceForDirectPurchaseTransitionV0 {
                    base: base.into(),
                    price,
                    public_note,
                },
            ),
        ))
    }

    #[wasm_bindgen(getter = base)]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = "price")]
    pub fn price(&self) -> Option<TokenPricingScheduleWasm> {
        self.0.price().map(|p| p.clone().into())
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = "price")]
    pub fn set_price(&mut self, price: Option<TokenPricingScheduleWasm>) {
        self.0.set_price(price.map(|p| p.into()));
    }
}

impl_try_from_js_value!(
    TokenSetPriceForDirectPurchaseTransitionWasm,
    "TokenSetPriceForDirectPurchaseTransition"
);
impl_wasm_type_info!(
    TokenSetPriceForDirectPurchaseTransitionWasm,
    TokenSetPriceForDirectPurchaseTransition
);
