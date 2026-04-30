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

impl<C> Decode<C> for Epoch {
    fn decode<D: bincode::de::Decoder<Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let index = EpochIndex::decode(decoder)?;
        Epoch::new(index).map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))
    }
}

impl<'de, C> BorrowDecode<'de, C> for Epoch {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let index = EpochIndex::borrow_decode(decoder)?;
        Epoch::new(index).map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))
    }
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for Epoch {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for Epoch {}


#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests_epoch {
    use super::*;

    fn fixture() -> Epoch {
        Epoch::new(7).expect("epoch")
    }

    fn assert_fields(e: &Epoch) {
        assert_eq!(e.index, 7, "index");
        // key is serde(skip) and reconstructed from index in Deserialize
        assert_eq!(e.key, Epoch::new(7).expect("epoch").key, "key matches Epoch::new(7)");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = Epoch::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = Epoch::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_fields(&recovered);
    }
}
