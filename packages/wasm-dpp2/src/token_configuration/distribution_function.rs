use crate::error::{WasmDppError, WasmDppResult};
use crate::token_configuration::distribution_structs::{
    DistributionExponentialWasm, DistributionFixedAmountWasm, DistributionInvertedLogarithmicWasm,
    DistributionLinearWasm, DistributionLogarithmicWasm, DistributionPolynomialWasm,
    DistributionRandomWasm, DistributionStepDecreasingAmountWasm,
};
use crate::utils::{JsValueExt, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use js_sys::{BigInt, Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "DistributionFunction")]
pub struct DistributionFunctionWasm(DistributionFunction);

impl From<DistributionFunctionWasm> for DistributionFunction {
    fn from(function: DistributionFunctionWasm) -> Self {
        function.0
    }
}

impl From<DistributionFunction> for DistributionFunctionWasm {
    fn from(function: DistributionFunction) -> Self {
        Self(function)
    }
}

#[wasm_bindgen(js_class = DistributionFunction)]
impl DistributionFunctionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DistributionFunction".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DistributionFunction".to_string()
    }

    #[wasm_bindgen(js_name = "FixedAmountDistribution")]
    pub fn fixed_amount_distribution(amount: TokenAmount) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::FixedAmount { amount })
    }

    #[wasm_bindgen(js_name = "Random")]
    pub fn random(min: TokenAmount, max: TokenAmount) -> Self {
        DistributionFunctionWasm(DistributionFunction::Random { min, max })
    }

    #[wasm_bindgen(js_name = "StepDecreasingAmount")]
    pub fn step_decreasing_amount(
        step_count: u32,
        decrease_per_interval_numerator: u16,
        decrease_per_interval_denominator: u16,
        start_decreasing_offset: Option<u64>,
        max_interval_count: Option<u16>,
        distribution_start_amount: TokenAmount,
        trailing_distribution_interval_amount: TokenAmount,
        min_value: Option<u64>,
    ) -> Self {
        DistributionFunctionWasm(DistributionFunction::StepDecreasingAmount {
            step_count,
            decrease_per_interval_numerator,
            decrease_per_interval_denominator,
            start_decreasing_offset,
            max_interval_count,
            distribution_start_amount,
            trailing_distribution_interval_amount,
            min_value,
        })
    }

    #[wasm_bindgen(js_name = "Stepwise")]
    pub fn stepwise(js_steps_with_amount: JsValue) -> WasmDppResult<DistributionFunctionWasm> {
        let obj = Object::from(js_steps_with_amount);

        let mut steps_with_amount: BTreeMap<u64, TokenAmount> = BTreeMap::new();

        for key in Object::keys(&obj) {
            let key_str = key
                .as_string()
                .ok_or_else(|| WasmDppError::invalid_argument("step key must be string"))?;

            let step = key_str.parse::<u64>().map_err(|err| {
                WasmDppError::invalid_argument(format!("Invalid step key '{}': {}", key_str, err))
            })?;

            let amount_js = Reflect::get(&obj, &key).map_err(|err| {
                let message = err.error_message();
                WasmDppError::invalid_argument(format!(
                    "unable to read distribution step '{}': {}",
                    key_str, message
                ))
            })?;

            let amount = try_to_u64(amount_js)
                .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

            steps_with_amount.insert(step, amount);
        }

        Ok(DistributionFunctionWasm(DistributionFunction::Stepwise(
            steps_with_amount,
        )))
    }

    #[wasm_bindgen(js_name = "Linear")]
    pub fn linear(
        a: i64,
        d: u64,
        start_step: Option<u64>,
        starting_amount: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    ) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::Linear {
            a,
            d,
            start_step,
            starting_amount,
            min_value,
            max_value,
        })
    }

    #[wasm_bindgen(js_name = "Polynomial")]
    pub fn polynomial(
        a: i64,
        d: u64,
        m: i64,
        n: u64,
        o: i64,
        start_moment: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    ) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::Polynomial {
            a,
            d,
            m,
            n,
            o,
            start_moment,
            b,
            min_value,
            max_value,
        })
    }

    #[wasm_bindgen(js_name = "Exponential")]
    pub fn exponential(
        a: u64,
        d: u64,
        m: i64,
        n: u64,
        o: i64,
        start_moment: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    ) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::Exponential {
            a,
            d,
            m,
            n,
            o,
            start_moment,
            b,
            min_value,
            max_value,
        })
    }

    #[wasm_bindgen(js_name = "Logarithmic")]
    pub fn logarithmic(
        a: i64,
        d: u64,
        m: u64,
        n: u64,
        o: i64,
        start_moment: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    ) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::Logarithmic {
            a,
            d,
            m,
            n,
            o,
            start_moment,
            b,
            min_value,
            max_value,
        })
    }

    #[wasm_bindgen(js_name = "InvertedLogarithmic")]
    pub fn inverted_logarithmic(
        a: i64,
        d: u64,
        m: u64,
        n: u64,
        o: i64,
        start_moment: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    ) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::InvertedLogarithmic {
            a,
            d,
            m,
            n,
            o,
            start_moment,
            b,
            min_value,
            max_value,
        })
    }

    #[wasm_bindgen(js_name = "getFunctionName")]
    pub fn get_function_name(&self) -> String {
        match self.0 {
            DistributionFunction::FixedAmount { .. } => String::from("FixedAmount"),
            DistributionFunction::Random { .. } => String::from("Random"),
            DistributionFunction::StepDecreasingAmount { .. } => {
                String::from("StepDecreasingAmount")
            }
            DistributionFunction::Stepwise(_) => String::from("Stepwise"),
            DistributionFunction::Linear { .. } => String::from("Linear"),
            DistributionFunction::Polynomial { .. } => String::from("Polynomial"),
            DistributionFunction::Exponential { .. } => String::from("Exponential"),
            DistributionFunction::Logarithmic { .. } => String::from("Logarithmic"),
            DistributionFunction::InvertedLogarithmic { .. } => String::from("InvertedLogarithmic"),
        }
    }

    #[wasm_bindgen(js_name = "getFunctionValue")]
    pub fn get_function_values(&self) -> WasmDppResult<JsValue> {
        match self.0.clone() {
            DistributionFunction::FixedAmount { amount } => {
                Ok(JsValue::from(DistributionFixedAmountWasm { amount }))
            }
            DistributionFunction::Random { min, max } => {
                Ok(JsValue::from(DistributionRandomWasm { min, max }))
            }
            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                start_decreasing_offset,
                max_interval_count,
                distribution_start_amount,
                trailing_distribution_interval_amount,
                min_value,
            } => Ok(JsValue::from(DistributionStepDecreasingAmountWasm {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                start_decreasing_offset,
                max_interval_count,
                distribution_start_amount,
                trailing_distribution_interval_amount,
                min_value,
            })),
            DistributionFunction::Stepwise(map) => {
                let object = Object::new();

                for (key, value) in map {
                    Reflect::set(
                        &object,
                        &key.to_string().into(),
                        &BigInt::from(value).into(),
                    )
                    .map_err(|err| {
                        let message = err.error_message();
                        WasmDppError::generic(format!(
                            "unable to serialize distribution function step '{}': {}",
                            key, message
                        ))
                    })?;
                }

                Ok(object.into())
            }
            DistributionFunction::Linear {
                a,
                d,
                start_step,
                starting_amount,
                min_value,
                max_value,
            } => Ok(JsValue::from(DistributionLinearWasm {
                a,
                d,
                start_step,
                starting_amount,
                min_value,
                max_value,
            })),
            DistributionFunction::Polynomial {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            } => Ok(JsValue::from(DistributionPolynomialWasm {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            })),
            DistributionFunction::Exponential {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            } => Ok(JsValue::from(DistributionExponentialWasm {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            })),
            DistributionFunction::Logarithmic {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            } => Ok(JsValue::from(DistributionLogarithmicWasm {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            })),
            DistributionFunction::InvertedLogarithmic {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            } => Ok(JsValue::from(DistributionInvertedLogarithmicWasm {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            })),
        }
    }
}
