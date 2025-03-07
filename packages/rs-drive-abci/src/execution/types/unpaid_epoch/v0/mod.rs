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
