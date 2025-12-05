pub mod deduct_fee_from_inputs_and_outputs;

pub use deduct_fee_from_inputs_and_outputs::FeeDeductionResult;

use bincode_derive::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum AddressFundsFeeStrategyStep {
    /// Deduct fee from a specific input address by index.
    /// The input must have remaining balance after its contribution to outputs.
    DeductFromInput(u16),
    /// Reduce a specific output by the fee amount.
    /// The output amount will be reduced to cover the fee.
    ReduceOutput(u16),
}

impl Default for AddressFundsFeeStrategyStep {
    fn default() -> Self {
        AddressFundsFeeStrategyStep::DeductFromInput(0)
    }
}

pub type AddressFundsFeeStrategy = Vec<AddressFundsFeeStrategyStep>;
