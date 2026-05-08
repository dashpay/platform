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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::fee::epoch::CreditsPerEpoch;
    use dpp::fee::fee_result::FeeResult;

    fn make_block_fees() -> BlockFeesV0 {
        BlockFeesV0 {
            processing_fee: 100,
            storage_fee: 200,
            refunds_per_epoch: CreditsPerEpoch::default(),
        }
    }

    #[test]
    fn block_fees_v0_getters_return_correct_values() {
        let fees = make_block_fees();
        assert_eq!(fees.processing_fee(), 100);
        assert_eq!(fees.storage_fee(), 200);
        assert!(fees.refunds_per_epoch().is_empty());
    }

    #[test]
    fn block_fees_v0_setters_update_values() {
        let mut fees = make_block_fees();
        fees.set_processing_fee(500);
        fees.set_storage_fee(600);
        assert_eq!(fees.processing_fee(), 500);
        assert_eq!(fees.storage_fee(), 600);

        let mut refunds = CreditsPerEpoch::default();
        refunds.insert(0, 42);
        fees.set_refunds_per_epoch(refunds);
        assert_eq!(*fees.refunds_per_epoch().get(&0).unwrap(), 42);
    }

    #[test]
    fn block_fees_v0_refunds_per_epoch_mut_allows_mutation() {
        let mut fees = make_block_fees();
        fees.refunds_per_epoch_mut().insert(5, 999);
        assert_eq!(*fees.refunds_per_epoch().get(&5).unwrap(), 999);
    }

    #[test]
    fn block_fees_v0_refunds_per_epoch_owned_consumes() {
        let mut fees = make_block_fees();
        fees.refunds_per_epoch_mut().insert(3, 77);
        let owned = fees.refunds_per_epoch_owned();
        assert_eq!(*owned.get(&3).unwrap(), 77);
    }

    #[test]
    fn block_fees_v0_from_fees_sets_correct_values() {
        let fees = BlockFeesV0::from_fees(300, 400);
        assert_eq!(fees.storage_fee(), 300);
        assert_eq!(fees.processing_fee(), 400);
        assert!(fees.refunds_per_epoch().is_empty());
    }

    #[test]
    fn block_fees_v0_default_is_zero() {
        let fees = BlockFeesV0::default();
        assert_eq!(fees.processing_fee(), 0);
        assert_eq!(fees.storage_fee(), 0);
        assert!(fees.refunds_per_epoch().is_empty());
    }

    #[test]
    fn block_fees_v0_from_fee_result() {
        let fee_result = FeeResult {
            storage_fee: 1000,
            processing_fee: 2000,
            ..Default::default()
        };
        let block_fees = BlockFeesV0::from(fee_result);
        assert_eq!(block_fees.storage_fee(), 1000);
        assert_eq!(block_fees.processing_fee(), 2000);
        assert!(block_fees.refunds_per_epoch().is_empty());
    }

    #[test]
    fn block_fees_v0_serialization_roundtrip() {
        let mut fees = make_block_fees();
        fees.refunds_per_epoch_mut().insert(1, 50);
        let serialized = serde_json::to_string(&fees).expect("serialization should succeed");
        let deserialized: BlockFeesV0 =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.processing_fee(), fees.processing_fee());
        assert_eq!(deserialized.storage_fee(), fees.storage_fee());
        assert_eq!(
            deserialized.refunds_per_epoch().get(&1),
            fees.refunds_per_epoch().get(&1)
        );
    }
}
