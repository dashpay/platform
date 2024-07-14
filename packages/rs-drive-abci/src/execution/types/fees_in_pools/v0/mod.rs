use dpp::fee::Credits;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Struct containing the amount of processing and storage fees in the distribution pools
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeesInPoolsV0 {
    /// Amount of processing fees in the distribution pools
    pub processing_fees: Credits,
    /// Amount of storage fees in the distribution pools
    pub storage_fees: Credits,
}

impl fmt::Display for FeesInPoolsV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "FeesInPoolsV0 {{")?;
        writeln!(f, "    processing_fees: {},", self.processing_fees)?;
        writeln!(f, "    storage_fees: {}", self.storage_fees)?;
        write!(f, "}}")
    }
}
