use dpp::fee::epoch::CreditsPerEpoch;
use dpp::fee::fee_result::FeeResult;
use serde::{Deserialize, Serialize};

/// Aggregated fees after block execution
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockFeesV0 {
    /// Processing fee
    pub processing_fee: u64,
    /// Storage fee
    pub storage_fee: u64,
    /// Fee refunds per epoch
    pub refunds_per_epoch: CreditsPerEpoch,
}

#[allow(dead_code)]
pub trait BlockFeesV0Methods {
    /// Create block fee result from fees
    fn from_fees(storage_fee: u64, processing_fee: u64) -> Self;
}

impl BlockFeesV0Methods for BlockFeesV0 {
    fn from_fees(storage_fee: u64, processing_fee: u64) -> Self {
        Self {
            storage_fee,
            processing_fee,
            ..Default::default()
        }
    }
}

/// `BlockFeesV0Getters` trait provides getter methods for `BlockFeesV0`.
#[allow(dead_code)]
pub trait BlockFeesV0Getters {
    /// Returns the processing fee.
    fn processing_fee(&self) -> u64;

    /// Returns the storage fee.
    fn storage_fee(&self) -> u64;

    /// Returns the fee refunds per epoch.
    fn refunds_per_epoch(&self) -> &CreditsPerEpoch;

    /// Returns the fee refunds per epoch.
    fn refunds_per_epoch_owned(self) -> CreditsPerEpoch;

    /// Returns the fee refunds per epoch.
    fn refunds_per_epoch_mut(&mut self) -> &mut CreditsPerEpoch;
}

/// `BlockFeesV0Setters` trait provides setter methods for `BlockFeesV0`.
#[allow(dead_code)]
pub trait BlockFeesV0Setters {
    /// Sets the processing fee.
    fn set_processing_fee(&mut self, fee: u64);

    /// Sets the storage fee.
    fn set_storage_fee(&mut self, fee: u64);

    /// Sets the fee refunds per epoch.
    fn set_refunds_per_epoch(&mut self, refunds: CreditsPerEpoch);
}

impl BlockFeesV0Getters for BlockFeesV0 {
    fn processing_fee(&self) -> u64 {
        self.processing_fee
    }

    fn storage_fee(&self) -> u64 {
        self.storage_fee
    }

    fn refunds_per_epoch(&self) -> &CreditsPerEpoch {
        &self.refunds_per_epoch
    }

    fn refunds_per_epoch_owned(self) -> CreditsPerEpoch {
        self.refunds_per_epoch
    }

    fn refunds_per_epoch_mut(&mut self) -> &mut CreditsPerEpoch {
        &mut self.refunds_per_epoch
    }
}

impl BlockFeesV0Setters for BlockFeesV0 {
    fn set_processing_fee(&mut self, fee: u64) {
        self.processing_fee = fee;
    }

    fn set_storage_fee(&mut self, fee: u64) {
        self.storage_fee = fee;
    }

    fn set_refunds_per_epoch(&mut self, refunds: CreditsPerEpoch) {
        self.refunds_per_epoch = refunds;
    }
}

impl From<FeeResult> for BlockFeesV0 {
    fn from(value: FeeResult) -> Self {
        Self {
            storage_fee: value.storage_fee,
            processing_fee: value.processing_fee,
            refunds_per_epoch: value.fee_refunds.sum_per_epoch(),
        }
    }
}
