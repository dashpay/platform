use crate::block::epoch::EpochIndex;
use crate::util::deserializer::ProtocolVersion;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Extended Epoch information
#[derive(Clone, Debug, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub struct ExtendedEpochInfoV0 {
    /// The index of the epoch
    pub index: EpochIndex,
    /// First block time
    pub first_block_time: u64,
    /// First block height
    pub first_block_height: u64,
    /// First core block height
    pub first_core_block_height: u32,
    /// Fee multiplier
    pub fee_multiplier: f64,
    /// Protocol version
    pub protocol_version: u32,
}

/// Trait defining getters for `ExtendedEpochInfoV0`.
pub trait ExtendedEpochInfoV0Getters {
    /// Returns the epoch index.
    fn index(&self) -> EpochIndex;

    /// Returns the first block time.
    fn first_block_time(&self) -> u64;

    /// Returns the first platform block height.
    fn first_block_height(&self) -> u64;

    /// Returns the first core block height.
    fn first_core_block_height(&self) -> u32;

    /// Returns the fee multiplier.
    fn fee_multiplier(&self) -> f64;

    /// Protocol version
    fn protocol_version(&self) -> ProtocolVersion;
}
