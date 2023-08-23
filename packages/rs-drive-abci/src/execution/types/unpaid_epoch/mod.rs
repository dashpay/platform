use crate::execution::types::unpaid_epoch::v0::{
    UnpaidEpochV0Getters, UnpaidEpochV0Methods, UnpaidEpochV0Setters,
};
use derive_more::From;
use dpp::block::epoch::EpochIndex;
use drive::error::Error;

pub mod v0;

/// UnpaidEpoch contains info about an epoch containing fees that have not been paid out yet.
#[derive(Debug, From)]
pub enum UnpaidEpoch {
    V0(v0::UnpaidEpochV0),
}

impl UnpaidEpochV0Methods for UnpaidEpoch {
    fn block_count(&self) -> Result<u64, Error> {
        match self {
            UnpaidEpoch::V0(v0) => v0.block_count(),
        }
    }
}

impl UnpaidEpochV0Getters for UnpaidEpoch {
    fn epoch_index(&self) -> EpochIndex {
        match self {
            UnpaidEpoch::V0(v0) => v0.epoch_index(),
        }
    }

    fn next_unpaid_epoch_index(&self) -> EpochIndex {
        match self {
            UnpaidEpoch::V0(v0) => v0.next_unpaid_epoch_index(),
        }
    }

    fn start_block_height(&self) -> u64 {
        match self {
            UnpaidEpoch::V0(v0) => v0.start_block_height(),
        }
    }

    fn next_epoch_start_block_height(&self) -> u64 {
        match self {
            UnpaidEpoch::V0(v0) => v0.next_epoch_start_block_height(),
        }
    }

    fn start_block_core_height(&self) -> u32 {
        match self {
            UnpaidEpoch::V0(v0) => v0.start_block_core_height(),
        }
    }

    fn next_epoch_start_block_core_height(&self) -> u32 {
        match self {
            UnpaidEpoch::V0(v0) => v0.next_epoch_start_block_core_height(),
        }
    }
}

impl UnpaidEpochV0Setters for UnpaidEpoch {
    fn set_epoch_index(&mut self, epoch_index: EpochIndex) {
        match self {
            UnpaidEpoch::V0(v0) => v0.set_epoch_index(epoch_index),
        }
    }

    fn set_next_unpaid_epoch_index(&mut self, next_unpaid_epoch_index: EpochIndex) {
        match self {
            UnpaidEpoch::V0(v0) => v0.set_next_unpaid_epoch_index(next_unpaid_epoch_index),
        }
    }

    fn set_start_block_height(&mut self, start_block_height: u64) {
        match self {
            UnpaidEpoch::V0(v0) => v0.set_start_block_height(start_block_height),
        }
    }

    fn set_next_epoch_start_block_height(&mut self, next_epoch_start_block_height: u64) {
        match self {
            UnpaidEpoch::V0(v0) => {
                v0.set_next_epoch_start_block_height(next_epoch_start_block_height)
            }
        }
    }

    fn set_start_block_core_height(&mut self, start_block_core_height: u32) {
        match self {
            UnpaidEpoch::V0(v0) => v0.set_start_block_core_height(start_block_core_height),
        }
    }

    fn set_next_epoch_start_block_core_height(&mut self, next_epoch_start_block_core_height: u32) {
        match self {
            UnpaidEpoch::V0(v0) => {
                v0.set_next_epoch_start_block_core_height(next_epoch_start_block_core_height)
            }
        }
    }
}
