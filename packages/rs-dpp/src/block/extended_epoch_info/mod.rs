pub mod v0;

use crate::block::epoch::EpochIndex;
use crate::block::extended_epoch_info::v0::{ExtendedEpochInfoV0, ExtendedEpochInfoV0Getters};
use crate::protocol_error::ProtocolError;
use crate::util::deserializer::ProtocolVersion;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};

/// Extended Block information
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    From,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub enum ExtendedEpochInfo {
    V0(ExtendedEpochInfoV0),
}

impl ExtendedEpochInfoV0Getters for ExtendedEpochInfo {
    fn index(&self) -> EpochIndex {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.index,
        }
    }

    fn first_block_time(&self) -> u64 {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.first_block_time,
        }
    }

    fn first_block_height(&self) -> u64 {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.first_block_height,
        }
    }

    fn first_core_block_height(&self) -> u32 {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.first_core_block_height,
        }
    }

    fn fee_multiplier(&self) -> f64 {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.fee_multiplier,
        }
    }

    fn protocol_version(&self) -> ProtocolVersion {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.protocol_version,
        }
    }
}
