use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use crate::utils::{IntoWasm, try_from_options_optional_with, try_to_array};
use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep};
use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use std::fmt;
use wasm_bindgen::prelude::*;

/// Defines how fees are paid in address-based state transitions.
///
/// Fee strategy is a sequence of steps that determine which inputs or outputs
/// should be reduced to cover the transaction fee.
#[wasm_bindgen(js_name = "FeeStrategyStep")]
#[derive(Clone, Debug)]
pub struct FeeStrategyStepWasm(AddressFundsFeeStrategyStep);

#[wasm_bindgen(js_class = FeeStrategyStep)]
impl FeeStrategyStepWasm {
    /// Creates a step that deducts the fee from the input at the given index.
    ///
    /// The input must have remaining balance after its contribution to outputs.
    ///
    /// @param index - The index of the input address to deduct fee from
    #[wasm_bindgen(js_name = "deductFromInput")]
    pub fn deduct_from_input(index: u16) -> FeeStrategyStepWasm {
        FeeStrategyStepWasm(AddressFundsFeeStrategyStep::DeductFromInput(index))
    }

    /// Creates a step that reduces the output at the given index by the fee amount.
    ///
    /// The output amount will be reduced to cover the fee.
    ///
    /// @param index - The index of the output address to reduce
    #[wasm_bindgen(js_name = "reduceOutput")]
    pub fn reduce_output(index: u16) -> FeeStrategyStepWasm {
        FeeStrategyStepWasm(AddressFundsFeeStrategyStep::ReduceOutput(index))
    }

    /// Returns true if this step deducts from an input.
    #[wasm_bindgen(js_name = "isDeductFromInput", getter)]
    pub fn is_deduct_from_input(&self) -> bool {
        matches!(self.0, AddressFundsFeeStrategyStep::DeductFromInput(_))
    }

    /// Returns true if this step reduces an output.
    #[wasm_bindgen(js_name = "isReduceOutput", getter)]
    pub fn is_reduce_output(&self) -> bool {
        matches!(self.0, AddressFundsFeeStrategyStep::ReduceOutput(_))
    }

    /// Returns the index associated with this step.
    #[wasm_bindgen(getter)]
    pub fn index(&self) -> u16 {
        match self.0 {
            AddressFundsFeeStrategyStep::DeductFromInput(i) => i,
            AddressFundsFeeStrategyStep::ReduceOutput(i) => i,
        }
    }
}

impl_wasm_type_info!(FeeStrategyStepWasm, FeeStrategyStep);

impl From<FeeStrategyStepWasm> for AddressFundsFeeStrategyStep {
    fn from(step: FeeStrategyStepWasm) -> Self {
        step.0
    }
}

impl From<AddressFundsFeeStrategyStep> for FeeStrategyStepWasm {
    fn from(step: AddressFundsFeeStrategyStep) -> Self {
        FeeStrategyStepWasm(step)
    }
}

/// Converts a vector of FeeStrategyStepWasm to AddressFundsFeeStrategy.
pub fn fee_strategy_from_steps(steps: Vec<FeeStrategyStepWasm>) -> AddressFundsFeeStrategy {
    steps.into_iter().map(|s| s.0).collect()
}

/// Returns the default fee strategy (deduct from first input).
pub fn default_fee_strategy() -> AddressFundsFeeStrategy {
    vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]
}

/// Converts optional fee strategy steps to AddressFundsFeeStrategy, using default if None.
pub fn fee_strategy_from_steps_or_default(
    steps: Option<Vec<FeeStrategyStepWasm>>,
) -> AddressFundsFeeStrategy {
    steps
        .map(fee_strategy_from_steps)
        .unwrap_or_else(default_fee_strategy)
}

/// Extract an optional Vec<FeeStrategyStepWasm> from a JS options object property.
///
/// Returns None if the property is undefined or null.
pub fn fee_strategy_from_js_options(
    options: &JsValue,
    field_name: &str,
) -> WasmDppResult<Option<Vec<FeeStrategyStepWasm>>> {
    try_from_options_optional_with(options, field_name, |v| {
        let array = try_to_array(v, field_name)?;
        array
            .iter()
            .enumerate()
            .map(|(i, item)| {
                item.to_wasm::<FeeStrategyStepWasm>("FeeStrategyStep")
                    .map(|r| (*r).clone())
                    .map_err(|_| {
                        WasmDppError::invalid_argument(format!(
                            "{}[{}] is not a FeeStrategyStep",
                            field_name, i
                        ))
                    })
            })
            .collect()
    })
}

impl<'de> Deserialize<'de> for FeeStrategyStepWasm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "camelCase")]
        enum Field {
            Type,
            Index,
        }

        struct FeeStrategyStepVisitor;

        impl<'de> Visitor<'de> for FeeStrategyStepVisitor {
            type Value = FeeStrategyStepWasm;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct FeeStrategyStep with type and index")
            }

            fn visit_map<V>(self, mut map: V) -> Result<FeeStrategyStepWasm, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut step_type: Option<String> = None;
                let mut index: Option<u16> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Type => {
                            if step_type.is_some() {
                                return Err(de::Error::duplicate_field("type"));
                            }
                            step_type = Some(map.next_value()?);
                        }
                        Field::Index => {
                            if index.is_some() {
                                return Err(de::Error::duplicate_field("index"));
                            }
                            index = Some(map.next_value()?);
                        }
                    }
                }

                let step_type = step_type.ok_or_else(|| de::Error::missing_field("type"))?;
                let index = index.ok_or_else(|| de::Error::missing_field("index"))?;

                match step_type.as_str() {
                    "deductFromInput" => Ok(FeeStrategyStepWasm::deduct_from_input(index)),
                    "reduceOutput" => Ok(FeeStrategyStepWasm::reduce_output(index)),
                    _ => Err(de::Error::unknown_variant(
                        &step_type,
                        &["deductFromInput", "reduceOutput"],
                    )),
                }
            }
        }

        const FIELDS: &[&str] = &["type", "index"];
        deserializer.deserialize_struct("FeeStrategyStep", FIELDS, FeeStrategyStepVisitor)
    }
}
