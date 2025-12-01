pub mod deduct_fee_from_inputs_and_outputs;

use bincode_derive::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum AddressFundsFeeStrategyStep {
    DeductFromInput(u16),
    ReduceOutput(u16),
}

impl Default for AddressFundsFeeStrategyStep {
    fn default() -> Self {
        AddressFundsFeeStrategyStep::DeductFromInput(0)
    }
}

pub type AddressFundsFeeStrategy = Vec<AddressFundsFeeStrategyStep>;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum AddressFundsFeeWithWithdrawalStrategyStep {
    DeductFromInput(u16),
    ReduceOutput(u16),
    ReduceWithdrawal,
}

impl Default for AddressFundsFeeWithWithdrawalStrategyStep {
    fn default() -> Self {
        AddressFundsFeeWithWithdrawalStrategyStep::ReduceWithdrawal
    }
}

pub type AddressFundsFeeWithWithdrawalsStrategy = Vec<AddressFundsFeeWithWithdrawalStrategyStep>;
