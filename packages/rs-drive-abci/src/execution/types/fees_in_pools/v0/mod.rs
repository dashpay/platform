use dpp::fee::Credits;
use serde::{Deserialize, Serialize};

/// Struct containing the amount of processing and storage fees in the distribution pools
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeesInPoolsV0 {
    /// Amount of processing fees in the distribution pools
    pub processing_fees: Credits,
    /// Amount of storage fees in the distribution pools
    pub storage_fees: Credits,
}
