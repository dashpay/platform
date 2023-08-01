use crate::ProtocolError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Epoch key offset
pub const EPOCH_KEY_OFFSET: u16 = 256;

/// Epoch index type
pub type EpochIndex = u16;

pub const EPOCH_0 : Epoch = Epoch {
    index: 0,
    key: [1,0],
};

// We make this immutable because it should never be changed or updated
// @immutable
/// Epoch struct
#[derive(Serialize, Deserialize, Default, Clone, Eq, PartialEq, Copy, Encode, Decode, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Epoch {
    /// Epoch index
    pub index: EpochIndex,

    /// Key
    // todo: don't serialize key
    pub key: [u8; 2],
}

impl Epoch {
    /// Create new epoch
    pub fn new(index: EpochIndex) -> Result<Self, ProtocolError> {
        let index_with_offset = index
            .checked_add(EPOCH_KEY_OFFSET)
            .ok_or(ProtocolError::Overflow("stored epoch index too high"))?;
        Ok(Self {
            index,
            key: index_with_offset.to_be_bytes(),
        })
    }
}
