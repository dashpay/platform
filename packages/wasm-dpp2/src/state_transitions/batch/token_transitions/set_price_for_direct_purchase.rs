use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::state_transitions::batch::token_pricing_schedule::TokenPricingScheduleWasm;
use crate::utils::IntoWasm;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransitionV0;
use dpp::state_transition::batch_transition::TokenSetPriceForDirectPurchaseTransition;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

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
        base: &TokenBaseTransitionWasm,
        price: &JsValue,
        #[wasm_bindgen(js_name = "publicNote")] public_note: Option<String>,
    ) -> WasmDppResult<TokenSetPriceForDirectPurchaseTransitionWasm> {
        let price: Option<TokenPricingSchedule> = if price.is_undefined() {
            None
        } else {
            Some(
                price
                    .to_wasm::<TokenPricingScheduleWasm>("TokenPricingSchedule")?
                    .clone()
                    .into(),
            )
        };

        Ok(TokenSetPriceForDirectPurchaseTransitionWasm(
            TokenSetPriceForDirectPurchaseTransition::V0(
                TokenSetPriceForDirectPurchaseTransitionV0 {
                    base: base.clone().into(),
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
