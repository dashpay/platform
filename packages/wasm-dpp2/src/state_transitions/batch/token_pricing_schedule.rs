use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use crate::utils::{JsValueExt, ToSerdeJSONExt};
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use js_sys::{Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "bigint | Record<string, bigint>")]
    pub type TokenPricingScheduleValueJs;
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TokenPricingSchedule")]
pub struct TokenPricingScheduleWasm(TokenPricingSchedule);

impl From<TokenPricingScheduleWasm> for TokenPricingSchedule {
    fn from(schedule: TokenPricingScheduleWasm) -> Self {
        schedule.0
    }
}

impl From<TokenPricingSchedule> for TokenPricingScheduleWasm {
    fn from(schedule: TokenPricingSchedule) -> Self {
        TokenPricingScheduleWasm(schedule)
    }
}

#[wasm_bindgen(js_class = TokenPricingSchedule)]
impl TokenPricingScheduleWasm {
    #[wasm_bindgen(js_name = "SinglePrice")]
    pub fn single_price(credits: Credits) -> Self {
        Self(TokenPricingSchedule::SinglePrice(credits))
    }

    #[wasm_bindgen(js_name = "SetPrices")]
    pub fn set_prices(
        #[wasm_bindgen(unchecked_param_type = "Record<string, bigint>")] prices: &JsValue,
    ) -> WasmDppResult<TokenPricingScheduleWasm> {
        let raw_prices = prices.with_serde_to_platform_value_map()?;

        let mut prices: BTreeMap<TokenAmount, Credits> = BTreeMap::new();

        for (amount_str, value) in raw_prices.iter() {
            let amount = amount_str.parse::<TokenAmount>().map_err(|err| {
                WasmDppError::invalid_argument(format!(
                    "Invalid token amount '{}': {}",
                    amount_str, err
                ))
            })?;

            let credits_value = value.as_integer::<u64>().ok_or_else(|| {
                WasmDppError::invalid_argument(format!(
                    "Price for amount '{}' must be an integer",
                    amount_str
                ))
            })?;

            prices.insert(amount, credits_value);
        }

        Ok(Self(TokenPricingSchedule::SetPrices(prices)))
    }

    #[wasm_bindgen(getter = "scheduleType")]
    pub fn schedule_type(&self) -> String {
        match &self.0 {
            TokenPricingSchedule::SinglePrice(_) => String::from("SinglePrice"),
            TokenPricingSchedule::SetPrices(_) => String::from("SetPrices"),
        }
    }

    #[wasm_bindgen(getter = "value")]
    pub fn value(&self) -> WasmDppResult<TokenPricingScheduleValueJs> {
        let js_value = match &self.0 {
            TokenPricingSchedule::SinglePrice(credits) => {
                JsValue::bigint_from_str(&credits.to_string())
            }
            TokenPricingSchedule::SetPrices(prices) => {
                let price_object = Object::new();

                for (key, value) in prices.iter() {
                    Reflect::set(
                        &price_object,
                        &JsValue::from(key.to_string()),
                        &JsValue::bigint_from_str(&value.to_string()),
                    )
                    .map_err(|err| {
                        let message = err.error_message();
                        WasmDppError::generic(format!(
                            "unable to serialize price for amount '{}': {}",
                            key, message
                        ))
                    })?;
                }

                price_object.into()
            }
        };
        Ok(js_value.into())
    }
}

impl_wasm_type_info!(TokenPricingScheduleWasm, TokenPricingSchedule);
