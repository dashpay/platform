use serde::{Deserialize, Serialize};
use drive::fee::credits::Credits;

/// Struct containing the amount of processing and storage fees in the distribution pools
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeesInPools {
    /// Amount of processing fees in the distribution pools
    pub processing_fees: Credits,
    /// Amount of storage fees in the distribution pools
    pub storage_fees: Credits,
}
