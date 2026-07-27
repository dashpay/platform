use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use crate::utils::{IntoWasm, try_from_options_optional_with, try_to_array};
use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const FEE_STRATEGY_STEP_TS_TYPES: &str = r#"
/**
 * Fee strategy step in Object form (output of a transition's `toObject()`).
 *
 * Discriminated by `type`: "deductFromInput" reduces an input's contribution
 * by the fee, "reduceOutput" reduces an output's amount by the fee. The
 * `index` selects the input/output position.
 */
export type FeeStrategyStepObject =
    | { $type: "deductFromInput"; index: number }
    | { $type: "reduceOutput"; index: number };

/**
 * Fee strategy step in JSON form (output of a transition's `toJSON()`).
 *
 * Identical shape to `FeeStrategyStepObject` because the only payload is a
 * small `index` (u16) which serializes the same way in both binary and
 * human-readable formats.
 */
export type FeeStrategyStepJSON =
    | { $type: "deductFromInput"; index: number }
    | { $type: "reduceOutput"; index: number };
"#;

/// Defines how fees are paid in address-based state transitions.
///
/// Fee strategy is a sequence of steps that determine which inputs or outputs
/// should be reduced to cover the transaction fee.
///
/// `#[serde(transparent)]` delegates to the inner `AddressFundsFeeStrategyStep`'s
/// custom serde, which produces the `{ $type, index }` adjacent shape used by
/// every wasm-sdk consumer that round-trips a `Vec<FeeStrategyStepWasm>`.
#[wasm_bindgen(js_name = "FeeStrategyStep")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
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
