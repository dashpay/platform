use crate::batch::token_base_transition::TokenBaseTransitionWASM;
use crate::batch::token_pricing_schedule::TokenPricingScheduleWASM;
use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransitionV0;
use dpp::state_transition::batch_transition::TokenSetPriceForDirectPurchaseTransition;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use crate::utils::IntoWasm;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenSetPriceForDirectPurchaseTransition)]
pub struct TokenSetPriceForDirectPurchaseTransitionWASM(TokenSetPriceForDirectPurchaseTransition);

impl From<TokenSetPriceForDirectPurchaseTransition>
    for TokenSetPriceForDirectPurchaseTransitionWASM
{
    fn from(transition: TokenSetPriceForDirectPurchaseTransition) -> Self {
        TokenSetPriceForDirectPurchaseTransitionWASM(transition)
    }
}

impl From<TokenSetPriceForDirectPurchaseTransitionWASM>
    for TokenSetPriceForDirectPurchaseTransition
{
    fn from(transition: TokenSetPriceForDirectPurchaseTransitionWASM) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenSetPriceForDirectPurchaseTransition)]
impl TokenSetPriceForDirectPurchaseTransitionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenSetPriceForDirectPurchaseTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenSetPriceForDirectPurchaseTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWASM,
        js_price: &JsValue,
        public_note: Option<String>,
    ) -> Result<TokenSetPriceForDirectPurchaseTransitionWASM, JsValue> {
        let price: Option<TokenPricingSchedule> = match js_price.is_undefined() {
            true => None,
            false => Some(
                js_price
                    .to_wasm::<TokenPricingScheduleWASM>("TokenPricingSchedule")?
                    .clone()
                    .into(),
            ),
        };

        Ok(TokenSetPriceForDirectPurchaseTransitionWASM(
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
    pub fn get_base(&self) -> TokenBaseTransitionWASM {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn get_public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = "price")]
    pub fn get_price(&self) -> JsValue {
        match self.0.price() {
            None => JsValue::null(),
            Some(price) => JsValue::from(TokenPricingScheduleWASM::from(price.clone())),
        }
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWASM) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = "price")]
    pub fn set_price(&mut self, js_price: &JsValue) -> Result<(), JsValue> {
        let price: Option<TokenPricingSchedule> = match js_price.is_undefined() {
            true => None,
            false => Some(
                js_price
                    .to_wasm::<TokenPricingScheduleWASM>("TokenPricingSchedule")?
                    .clone()
                    .into(),
            ),
        };

        Ok(self.0.set_price(price))
    }
}
