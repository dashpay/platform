pub mod v0;

use crate::identifier::Identifier;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};
use v0::{EvonodeStatusV0, EvonodeStatusV0Getters};

/// Information about the status of an Evonode
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
)]
pub enum EvonodeStatus {
    V0(EvonodeStatusV0),
}

impl EvonodeStatusV0Getters for EvonodeStatus {
    /// Returns the Evonode Identifier
    fn pro_tx_hash(&self) -> String {
        match self {
            EvonodeStatus::V0(v0) => v0.pro_tx_hash.clone(),
        }
    }

    /// Returns the Evonode's latest stored block height
    fn latest_block_height(&self) -> u64 {
        match self {
            EvonodeStatus::V0(v0) => v0.latest_block_height,
        }
    }
}
