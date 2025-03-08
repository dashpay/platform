use crate::execution::types::unpaid_epoch::v0::{
    UnpaidEpochV0Getters, UnpaidEpochV0Methods, UnpaidEpochV0Setters,
};
use derive_more::From;
use dpp::block::epoch::EpochIndex;
use dpp::identity::TimestampMillis;
use dpp::prelude::FeeMultiplier;
use dpp::util::deserializer::ProtocolVersion;
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

    fn epoch_start_time(&self) -> TimestampMillis {
        match self {
            UnpaidEpoch::V0(v0) => v0.epoch_start_time(),
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

    fn protocol_version(&self) -> ProtocolVersion {
        match self {
            UnpaidEpoch::V0(v0) => v0.protocol_version(),
        }
    }

    fn fee_multiplier(&self) -> FeeMultiplier {
        match self {
            UnpaidEpoch::V0(v0) => v0.fee_multiplier(),
        }
    }
}

impl UnpaidEpochV0Setters for UnpaidEpoch {
    fn set_epoch_index(&mut self, epoch_index: EpochIndex) {
        match self {
            UnpaidEpoch::V0(v0) => v0.set_epoch_index(epoch_index),
        }
    }

    fn set_epoch_start_time(&mut self, epoch_start_time: TimestampMillis) {
        match self {
            UnpaidEpoch::V0(v0) => v0.set_epoch_start_time(epoch_start_time),
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

    fn set_protocol_version(&mut self, protocol_version: ProtocolVersion) {
        match self {
            UnpaidEpoch::V0(v0) => v0.set_protocol_version(protocol_version),
        }
    }

    fn set_fee_multiplier(&mut self, fee_multiplier: FeeMultiplier) {
        match self {
            UnpaidEpoch::V0(v0) => v0.set_fee_multiplier(fee_multiplier),
        }
    }
}
