use crate::{InvalidVectorSizeError, ProtocolError};
use bincode::{BorrowDecode, Decode, Encode};
use serde::{Deserialize, Serialize};

/// Epoch key offset
pub const EPOCH_KEY_OFFSET: u16 = 256;

/// The Highest allowed Epoch
pub const MAX_EPOCH: u16 = u16::MAX - EPOCH_KEY_OFFSET;

/// Epoch index type
pub type EpochIndex = u16;

pub const EPOCH_0: Epoch = Epoch {
    index: 0,
    key: [1, 0],
};

// We make this immutable because it should never be changed or updated
// @immutable
/// Epoch struct
#[derive(Serialize, Clone, Eq, PartialEq, Copy, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Epoch {
    /// Epoch index
    pub index: EpochIndex,

    /// Key
    #[serde(skip)]
    pub key: [u8; 2],
}

impl Default for Epoch {
    fn default() -> Self {
        Self::new(0).unwrap()
    }
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

impl TryFrom<EpochIndex> for Epoch {
    type Error = ProtocolError;

    fn try_from(value: EpochIndex) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&Vec<u8>> for Epoch {
    type Error = ProtocolError;

    fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
        let key = value.clone().try_into().map_err(|_| {
            ProtocolError::InvalidVectorSizeError(InvalidVectorSizeError::new(2, value.len()))
        })?;
        let index_with_offset = u16::from_be_bytes(key);
        let index = index_with_offset
            .checked_sub(EPOCH_KEY_OFFSET)
            .ok_or(ProtocolError::Overflow("value too low, must have offset"))?;
        Ok(Epoch { index, key })
    }
}

impl Encode for Epoch {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.index.encode(encoder)
    }
}

impl<'de> Deserialize<'de> for Epoch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct EpochData {
            index: EpochIndex,
        }

        let data = EpochData::deserialize(deserializer)?;
        Epoch::new(data.index).map_err(serde::de::Error::custom)
    }
}

impl Decode for Epoch {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let index = EpochIndex::decode(decoder)?;
        Epoch::new(index).map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))
    }
}

impl<'de> BorrowDecode<'de> for Epoch {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let index = EpochIndex::borrow_decode(decoder)?;
        Epoch::new(index).map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))
    }
}
