use dpp::block::epoch::EpochIndex;
use drive::error;
use drive::error::fee::FeeError;

/// Struct containing info about an epoch containing fees that have not been paid out yet.
#[derive(Default, PartialEq, Eq)]
pub struct UnpaidEpoch {
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

impl UnpaidEpoch {
    /// Counts and returns the number of blocks in the epoch
    pub fn block_count(&self) -> Result<u64, error::Error> {
        self.next_epoch_start_block_height
            .checked_sub(self.start_block_height)
            .ok_or(error::Error::Fee(FeeError::Overflow(
                "overflow for get_epoch_block_count",
            )))
    }
}
