use bincode_derive::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
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
