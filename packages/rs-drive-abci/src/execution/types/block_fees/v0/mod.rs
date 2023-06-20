use drive::fee::epoch::CreditsPerEpoch;
use drive::fee::result::FeeResult;
use serde::{Deserialize, Serialize};

/// Aggregated fees after block execution
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockFees {
    /// Processing fee
    pub processing_fee: u64,
    /// Storage fee
    pub storage_fee: u64,
    /// Fee refunds per epoch
    pub refunds_per_epoch: CreditsPerEpoch,
}

impl BlockFees {
    /// Create block fee result from fees
    pub fn from_fees(storage_fee: u64, processing_fee: u64) -> Self {
        Self {
            storage_fee,
            processing_fee,
            ..Default::default()
        }
    }
}

impl From<FeeResult> for BlockFees {
    fn from(value: FeeResult) -> Self {
        Self {
            storage_fee: value.storage_fee,
            processing_fee: value.processing_fee,
            refunds_per_epoch: value.fee_refunds.sum_per_epoch(),
        }
    }
}
