use dpp::block::epoch::EpochIndex;
use drive::error;
use drive::error::fee::FeeError;

/// Struct containing info about an epoch containing fees that have not been paid out yet.
#[derive(Default, PartialEq, Eq, Debug)]
pub struct UnpaidEpochV0 {
    /// Index of the current epoch
    pub epoch_index: EpochIndex,
    /// Index of the next unpaid epoch
    pub next_unpaid_epoch_index: EpochIndex,
    /// Block height of the first block in the epoch
    pub start_block_height: u64,
    /// Block height of the first block in next epoch
    pub next_epoch_start_block_height: u64,
    /// Block height of the first block in the epoch
    pub start_block_core_height: u32,
    /// Block height of the first block in next epoch
    pub next_epoch_start_block_core_height: u32,
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
    /// Get the index of the next unpaid epoch
    fn next_unpaid_epoch_index(&self) -> EpochIndex;
    /// Get the block height of the first block in the epoch
    fn start_block_height(&self) -> u64;
    /// Get the block height of the first block in the next epoch
    fn next_epoch_start_block_height(&self) -> u64;
    /// Get the block height of the first block in the epoch in the core chain
    fn start_block_core_height(&self) -> u32;
    /// Get the block height of the first block in the next epoch in the core chain
    fn next_epoch_start_block_core_height(&self) -> u32;
}

/// Trait that defines setter methods for `UnpaidEpochV0`
pub trait UnpaidEpochV0Setters {
    /// Set the index of the current epoch
    fn set_epoch_index(&mut self, epoch_index: EpochIndex);
    /// Set the index of the next unpaid epoch
    fn set_next_unpaid_epoch_index(&mut self, next_unpaid_epoch_index: EpochIndex);
    /// Set the block height of the first block in the epoch
    fn set_start_block_height(&mut self, start_block_height: u64);
    /// Set the block height of the first block in the next epoch
    fn set_next_epoch_start_block_height(&mut self, next_epoch_start_block_height: u64);
    /// Set the block height of the first block in the epoch in the core chain
    fn set_start_block_core_height(&mut self, start_block_core_height: u32);
    /// Set the block height of the first block in the next epoch in the core chain
    fn set_next_epoch_start_block_core_height(&mut self, next_epoch_start_block_core_height: u32);
}

impl UnpaidEpochV0Getters for UnpaidEpochV0 {
    fn epoch_index(&self) -> EpochIndex {
        self.epoch_index
    }

    fn next_unpaid_epoch_index(&self) -> EpochIndex {
        self.next_unpaid_epoch_index
    }

    fn start_block_height(&self) -> u64 {
        self.start_block_height
    }

    fn next_epoch_start_block_height(&self) -> u64 {
        self.next_epoch_start_block_height
    }

    fn start_block_core_height(&self) -> u32 {
        self.start_block_core_height
    }

    fn next_epoch_start_block_core_height(&self) -> u32 {
        self.next_epoch_start_block_core_height
    }
}

impl UnpaidEpochV0Setters for UnpaidEpochV0 {
    fn set_epoch_index(&mut self, epoch_index: EpochIndex) {
        self.epoch_index = epoch_index;
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
}
