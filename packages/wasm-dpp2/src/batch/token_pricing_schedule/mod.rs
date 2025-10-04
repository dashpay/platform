use crate::utils::ToSerdeJSONExt;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use js_sys::{Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TokenPricingSchedule")]
pub struct TokenPricingScheduleWASM(TokenPricingSchedule);

impl From<TokenPricingScheduleWASM> for TokenPricingSchedule {
    fn from(schedule: TokenPricingScheduleWASM) -> Self {
        schedule.0
    }
}

impl From<TokenPricingSchedule> for TokenPricingScheduleWASM {
    fn from(schedule: TokenPricingSchedule) -> Self {
        TokenPricingScheduleWASM(schedule)
    }
}

#[wasm_bindgen(js_class = TokenPricingSchedule)]
impl TokenPricingScheduleWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenPricingSchedule".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenPricingSchedule".to_string()
    }

    #[wasm_bindgen(js_name = "SinglePrice")]
    pub fn single_price(credits: Credits) -> Self {
        Self(TokenPricingSchedule::SinglePrice(credits))
    }

    #[wasm_bindgen(js_name = "SetPrices")]
    pub fn set_prices(js_prices: &JsValue) -> Result<TokenPricingScheduleWASM, JsValue> {
        let prices: BTreeMap<TokenAmount, Credits> = js_prices
            .with_serde_to_platform_value_map()?
            .iter()
            .map(|(k, v)| (k.clone().parse().unwrap(), v.clone().as_integer().unwrap()))
            .collect();

        Ok(Self(TokenPricingSchedule::SetPrices(prices)))
    }

    #[wasm_bindgen(js_name = "getScheduleType")]
    pub fn get_scheduled_type(&self) -> String {
        match &self.0 {
            TokenPricingSchedule::SinglePrice(_) => String::from("SinglePrice"),
            TokenPricingSchedule::SetPrices(_) => String::from("SetPrices"),
        }
    }

    #[wasm_bindgen(js_name = "getValue")]
    pub fn get_value(&self) -> Result<JsValue, JsValue> {
        match &self.0 {
            TokenPricingSchedule::SinglePrice(credits) => {
                Ok(JsValue::bigint_from_str(&credits.to_string()))
            }
            TokenPricingSchedule::SetPrices(prices) => {
                let price_object = Object::new();

                for (key, value) in prices.iter() {
                    Reflect::set(
                        &price_object,
                        &JsValue::from(key.to_string()),
                        &value.clone().into(),
                    )?;
                }

                Ok(price_object.into())
            }
        }
    }
}
