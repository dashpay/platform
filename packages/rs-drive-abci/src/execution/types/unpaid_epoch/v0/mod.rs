use dpp::block::epoch::EpochIndex;
use dpp::identity::TimestampMillis;
use dpp::prelude::{BlockHeight, CoreBlockHeight, FeeMultiplier};
use dpp::util::deserializer::ProtocolVersion;
use drive::error;
use drive::error::fee::FeeError;

/// Struct containing info about an epoch containing fees that have not been paid out yet.
#[derive(Default, PartialEq, Eq, Debug)]
pub struct UnpaidEpochV0 {
    /// Index of the current epoch
    pub epoch_index: EpochIndex,
    /// Index of the next unpaid epoch
    pub next_unpaid_epoch_index: EpochIndex,
    /// Start time of the epoch
    /// Also the time that the first block of the epoch was created
    pub epoch_start_time: TimestampMillis,
    /// Block height of the first block in the epoch
    pub start_block_height: BlockHeight,
    /// Block height of the first block in next epoch
    pub next_epoch_start_block_height: BlockHeight,
    /// Block height of the first block in the epoch
    pub start_block_core_height: CoreBlockHeight,
    /// Block height of the first block in next epoch
    pub next_epoch_start_block_core_height: CoreBlockHeight,
    /// Protocol version
    pub protocol_version: ProtocolVersion,
    /// Fee multiplier
    pub fee_multiplier: FeeMultiplier,
}

pub trait UnpaidEpochV0Methods {
    /// Counts and returns the number of blocks in the epoch
    fn block_count(&self) -> Result<u64, error::Error>;
}

impl UnpaidEpochV0Methods for UnpaidEpochV0 {
    /// Counts and returns the number of blocks in the epoch
    fn block_count(&self) -> Result<u64, error::Error> {
        self.next_epoch_start_block_height
            .checked_sub(self.start_block_height)
            .ok_or(error::Error::Fee(FeeError::Overflow(
                "overflow for get_epoch_block_count",
            )))
    }
}

/// Trait that defines getter methods for `UnpaidEpochV0`
#[allow(dead_code)]
pub trait UnpaidEpochV0Getters {
    /// Get the index of the current epoch
    fn epoch_index(&self) -> EpochIndex;
    /// Get the start time of the epoch
    fn epoch_start_time(&self) -> TimestampMillis;
    /// Get the index of the next unpaid epoch
    fn next_unpaid_epoch_index(&self) -> EpochIndex;
    /// Get the block height of the first block in the epoch
    fn start_block_height(&self) -> BlockHeight;
    /// Get the block height of the first block in the next epoch
    fn next_epoch_start_block_height(&self) -> BlockHeight;
    /// Get the block height of the first block in the epoch in the core chain
    fn start_block_core_height(&self) -> CoreBlockHeight;
    /// Get the block height of the first block in the next epoch in the core chain
    fn next_epoch_start_block_core_height(&self) -> CoreBlockHeight;

    /// Get the protocol version that the epoch used
    fn protocol_version(&self) -> ProtocolVersion;

    /// Get the fee multiplier that the epoch used
    fn fee_multiplier(&self) -> FeeMultiplier;
}

/// Trait that defines setter methods for `UnpaidEpochV0`
#[allow(dead_code)]
pub trait UnpaidEpochV0Setters {
    /// Set the index of the current epoch
    fn set_epoch_index(&mut self, epoch_index: EpochIndex);
    /// Set the start time of the epoch
    fn set_epoch_start_time(&mut self, epoch_start_time: TimestampMillis);
    /// Set the index of the next unpaid epoch
    fn set_next_unpaid_epoch_index(&mut self, next_unpaid_epoch_index: EpochIndex);
    /// Set the block height of the first block in the epoch
    fn set_start_block_height(&mut self, start_block_height: BlockHeight);
    /// Set the block height of the first block in the next epoch
    fn set_next_epoch_start_block_height(&mut self, next_epoch_start_block_height: BlockHeight);
    /// Set the block height of the first block in the epoch in the core chain
    fn set_start_block_core_height(&mut self, start_block_core_height: CoreBlockHeight);
    /// Set the block height of the first block in the next epoch in the core chain
    fn set_next_epoch_start_block_core_height(
        &mut self,
        next_epoch_start_block_core_height: CoreBlockHeight,
    );

    /// Set the protocol version that the epoch used
    fn set_protocol_version(&mut self, protocol_version: ProtocolVersion);

    /// Set the fee multiplier that the epoch used
    fn set_fee_multiplier(&mut self, fee_multiplier: FeeMultiplier);
}

impl UnpaidEpochV0Getters for UnpaidEpochV0 {
    fn epoch_index(&self) -> EpochIndex {
        self.epoch_index
    }

    fn epoch_start_time(&self) -> TimestampMillis {
        self.epoch_start_time
    }

    fn next_unpaid_epoch_index(&self) -> EpochIndex {
        self.next_unpaid_epoch_index
    }

    fn start_block_height(&self) -> BlockHeight {
        self.start_block_height
    }

    fn next_epoch_start_block_height(&self) -> BlockHeight {
        self.next_epoch_start_block_height
    }

    fn start_block_core_height(&self) -> CoreBlockHeight {
        self.start_block_core_height
    }

    fn next_epoch_start_block_core_height(&self) -> CoreBlockHeight {
        self.next_epoch_start_block_core_height
    }

    fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    fn fee_multiplier(&self) -> FeeMultiplier {
        self.fee_multiplier
    }
}

impl UnpaidEpochV0Setters for UnpaidEpochV0 {
    fn set_epoch_index(&mut self, epoch_index: EpochIndex) {
        self.epoch_index = epoch_index;
    }

    fn set_epoch_start_time(&mut self, epoch_start_time: TimestampMillis) {
        self.epoch_start_time = epoch_start_time;
    }

    fn set_next_unpaid_epoch_index(&mut self, next_unpaid_epoch_index: EpochIndex) {
        self.next_unpaid_epoch_index = next_unpaid_epoch_index;
    }

    fn set_start_block_height(&mut self, start_block_height: u64) {
        self.start_block_height = start_block_height;
    }

    fn set_next_epoch_start_block_height(&mut self, next_epoch_start_block_height: u64) {
        self.next_epoch_start_block_height = next_epoch_start_block_height;
    }

    fn set_start_block_core_height(&mut self, start_block_core_height: u32) {
        self.start_block_core_height = start_block_core_height;
    }

    fn set_next_epoch_start_block_core_height(&mut self, next_epoch_start_block_core_height: u32) {
        self.next_epoch_start_block_core_height = next_epoch_start_block_core_height;
    }

    fn set_protocol_version(&mut self, protocol_version: ProtocolVersion) {
        self.protocol_version = protocol_version;
    }

    fn set_fee_multiplier(&mut self, fee_multiplier: FeeMultiplier) {
        self.fee_multiplier = fee_multiplier;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_unpaid_epoch() -> UnpaidEpochV0 {
        UnpaidEpochV0 {
            epoch_index: 5,
            next_unpaid_epoch_index: 6,
            epoch_start_time: 1_000_000,
            start_block_height: 100,
            next_epoch_start_block_height: 200,
            start_block_core_height: 50,
            next_epoch_start_block_core_height: 60,
            protocol_version: 1,
            fee_multiplier: 150,
        }
    }

    #[test]
    fn getters_return_correct_values() {
        let epoch = make_unpaid_epoch();
        assert_eq!(epoch.epoch_index(), 5);
        assert_eq!(epoch.next_unpaid_epoch_index(), 6);
        assert_eq!(epoch.epoch_start_time(), 1_000_000);
        assert_eq!(epoch.start_block_height(), 100);
        assert_eq!(epoch.next_epoch_start_block_height(), 200);
        assert_eq!(epoch.start_block_core_height(), 50);
        assert_eq!(epoch.next_epoch_start_block_core_height(), 60);
        assert_eq!(epoch.protocol_version(), 1);
        assert_eq!(epoch.fee_multiplier(), 150);
    }

    #[test]
    fn setters_update_all_fields() {
        let mut epoch = UnpaidEpochV0::default();
        epoch.set_epoch_index(10);
        epoch.set_next_unpaid_epoch_index(11);
        epoch.set_epoch_start_time(2_000_000);
        epoch.set_start_block_height(500);
        epoch.set_next_epoch_start_block_height(600);
        epoch.set_start_block_core_height(100);
        epoch.set_next_epoch_start_block_core_height(110);
        epoch.set_protocol_version(2);
        epoch.set_fee_multiplier(200);

        assert_eq!(epoch.epoch_index(), 10);
        assert_eq!(epoch.next_unpaid_epoch_index(), 11);
        assert_eq!(epoch.epoch_start_time(), 2_000_000);
        assert_eq!(epoch.start_block_height(), 500);
        assert_eq!(epoch.next_epoch_start_block_height(), 600);
        assert_eq!(epoch.start_block_core_height(), 100);
        assert_eq!(epoch.next_epoch_start_block_core_height(), 110);
        assert_eq!(epoch.protocol_version(), 2);
        assert_eq!(epoch.fee_multiplier(), 200);
    }

    #[test]
    fn block_count_returns_difference() {
        let epoch = make_unpaid_epoch();
        // next_epoch_start_block_height(200) - start_block_height(100) = 100
        assert_eq!(epoch.block_count().unwrap(), 100);
    }

    #[test]
    fn block_count_returns_zero_for_same_heights() {
        let mut epoch = make_unpaid_epoch();
        epoch.set_start_block_height(100);
        epoch.set_next_epoch_start_block_height(100);
        assert_eq!(epoch.block_count().unwrap(), 0);
    }

    #[test]
    fn block_count_returns_error_on_underflow() {
        let mut epoch = make_unpaid_epoch();
        epoch.set_start_block_height(200);
        epoch.set_next_epoch_start_block_height(100);
        assert!(epoch.block_count().is_err());
    }

    #[test]
    fn default_values_are_zero() {
        let epoch = UnpaidEpochV0::default();
        assert_eq!(epoch.epoch_index(), 0);
        assert_eq!(epoch.next_unpaid_epoch_index(), 0);
        assert_eq!(epoch.epoch_start_time(), 0);
        assert_eq!(epoch.start_block_height(), 0);
        assert_eq!(epoch.next_epoch_start_block_height(), 0);
        assert_eq!(epoch.start_block_core_height(), 0);
        assert_eq!(epoch.next_epoch_start_block_core_height(), 0);
        assert_eq!(epoch.protocol_version(), 0);
        assert_eq!(epoch.fee_multiplier(), 0);
    }
}
