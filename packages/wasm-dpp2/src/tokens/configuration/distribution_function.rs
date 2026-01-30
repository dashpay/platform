use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_from_for_extern_type;
use crate::impl_wasm_type_info;
use crate::tokens::configuration::distribution_structs::{
    DistributionExponentialWasm, DistributionFixedAmountWasm, DistributionInvertedLogarithmicWasm,
    DistributionLinearWasm, DistributionLogarithmicWasm, DistributionPolynomialWasm,
    DistributionRandomWasm, DistributionStepDecreasingAmountWasm,
};
use crate::utils::{JsValueExt, try_to_object, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use js_sys::{BigInt, Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        typescript_type = "DistributionFixedAmount | DistributionRandom | DistributionStepDecreasingAmount | Record<string, bigint> | DistributionLinear | DistributionPolynomial | DistributionExponential | DistributionLogarithmic | DistributionInvertedLogarithmic"
    )]
    pub type DistributionFunctionValue;
}

// Source types only (wasm_bindgen provides From<JsValue>)
impl_from_for_extern_type!(
    DistributionFunctionValue,
    DistributionFixedAmountWasm,
    DistributionRandomWasm,
    DistributionStepDecreasingAmountWasm,
    DistributionLinearWasm,
    DistributionPolynomialWasm,
    DistributionExponentialWasm,
    DistributionLogarithmicWasm,
    DistributionInvertedLogarithmicWasm,
    Object,
);

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
    #[wasm_bindgen(js_name = "FixedAmountDistribution")]
    pub fn fixed_amount_distribution(opts: DistributionFixedAmountWasm) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::FixedAmount {
            amount: opts.amount,
        })
    }

    #[wasm_bindgen(js_name = "Random")]
    pub fn random(opts: DistributionRandomWasm) -> Self {
        DistributionFunctionWasm(DistributionFunction::Random {
            min: opts.min,
            max: opts.max,
        })
    }

    #[wasm_bindgen(js_name = "StepDecreasingAmount")]
    pub fn step_decreasing_amount(opts: DistributionStepDecreasingAmountWasm) -> Self {
        DistributionFunctionWasm(DistributionFunction::StepDecreasingAmount {
            step_count: opts.step_count,
            decrease_per_interval_numerator: opts.decrease_per_interval_numerator,
            decrease_per_interval_denominator: opts.decrease_per_interval_denominator,
            start_decreasing_offset: opts.start_decreasing_offset,
            max_interval_count: opts.max_interval_count,
            distribution_start_amount: opts.distribution_start_amount,
            trailing_distribution_interval_amount: opts.trailing_distribution_interval_amount,
            min_value: opts.min_value,
        })
    }

    #[wasm_bindgen(js_name = "Stepwise")]
    pub fn stepwise(
        #[wasm_bindgen(unchecked_param_type = "Record<string, bigint>")] steps_with_amount: JsValue,
    ) -> WasmDppResult<DistributionFunctionWasm> {
        let obj = try_to_object(steps_with_amount, "stepsWithAmount")?;

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

            let amount = try_to_u64(&amount_js, &format!("step[{}]", key_str))?;

            steps_with_amount.insert(step, amount);
        }

        Ok(DistributionFunctionWasm(DistributionFunction::Stepwise(
            steps_with_amount,
        )))
    }

    #[wasm_bindgen(js_name = "Linear")]
    pub fn linear(opts: DistributionLinearWasm) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::Linear {
            a: opts.a,
            d: opts.d,
            start_step: opts.start_step,
            starting_amount: opts.starting_amount,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(js_name = "Polynomial")]
    pub fn polynomial(opts: DistributionPolynomialWasm) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::Polynomial {
            a: opts.a,
            d: opts.d,
            m: opts.m,
            n: opts.n,
            o: opts.o,
            start_moment: opts.start_moment,
            b: opts.b,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(js_name = "Exponential")]
    pub fn exponential(opts: DistributionExponentialWasm) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::Exponential {
            a: opts.a,
            d: opts.d,
            m: opts.m,
            n: opts.n,
            o: opts.o,
            start_moment: opts.start_moment,
            b: opts.b,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(js_name = "Logarithmic")]
    pub fn logarithmic(opts: DistributionLogarithmicWasm) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::Logarithmic {
            a: opts.a,
            d: opts.d,
            m: opts.m,
            n: opts.n,
            o: opts.o,
            start_moment: opts.start_moment,
            b: opts.b,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(js_name = "InvertedLogarithmic")]
    pub fn inverted_logarithmic(
        opts: DistributionInvertedLogarithmicWasm,
    ) -> DistributionFunctionWasm {
        DistributionFunctionWasm(DistributionFunction::InvertedLogarithmic {
            a: opts.a,
            d: opts.d,
            m: opts.m,
            n: opts.n,
            o: opts.o,
            start_moment: opts.start_moment,
            b: opts.b,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(getter = "functionName")]
    pub fn function_name(&self) -> String {
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

    #[wasm_bindgen(getter = "functionValue")]
    pub fn function_value(&self) -> WasmDppResult<DistributionFunctionValue> {
        let js_value: JsValue = match self.0.clone() {
            DistributionFunction::FixedAmount { amount } => {
                DistributionFixedAmountWasm { amount }.into()
            }
            DistributionFunction::Random { min, max } => DistributionRandomWasm { min, max }.into(),
            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                start_decreasing_offset,
                max_interval_count,
                distribution_start_amount,
                trailing_distribution_interval_amount,
                min_value,
            } => DistributionStepDecreasingAmountWasm {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                start_decreasing_offset,
                max_interval_count,
                distribution_start_amount,
                trailing_distribution_interval_amount,
                min_value,
            }
            .into(),
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

                object.into()
            }
            DistributionFunction::Linear {
                a,
                d,
                start_step,
                starting_amount,
                min_value,
                max_value,
            } => DistributionLinearWasm {
                a,
                d,
                start_step,
                starting_amount,
                min_value,
                max_value,
            }
            .into(),
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
            } => DistributionPolynomialWasm {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            }
            .into(),
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
            } => DistributionExponentialWasm {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            }
            .into(),
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
            } => DistributionLogarithmicWasm {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            }
            .into(),
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
            } => DistributionInvertedLogarithmicWasm {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            }
            .into(),
        };
        Ok(js_value.into())
    }
}

impl_wasm_type_info!(DistributionFunctionWasm, DistributionFunction);
