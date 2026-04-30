use crate::consensus::basic::data_contract::UnknownStorageKeyRequirementsError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use serde_repr::*;
use std::convert::TryFrom;

/// The Storage Key requirements
// @append_only
#[repr(u8)]
#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Eq, Copy, Clone, Encode, Decode)]
pub enum StorageKeyRequirements {
    Unique = 0,
    Multiple = 1,
    MultipleReferenceToLatest = 2,
}

impl TryFrom<u8> for StorageKeyRequirements {
    type Error = ProtocolError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unique),
            1 => Ok(Self::Multiple),
            2 => Ok(Self::MultipleReferenceToLatest),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownStorageKeyRequirementsError(
                    UnknownStorageKeyRequirementsError::new(vec![0, 1, 3], value.into()),
                ))
                .into(),
            )),
        }
    }
}

impl TryFrom<i128> for StorageKeyRequirements {
    type Error = ProtocolError;
    fn try_from(value: i128) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unique),
            1 => Ok(Self::Multiple),
            2 => Ok(Self::MultipleReferenceToLatest),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownStorageKeyRequirementsError(
                    UnknownStorageKeyRequirementsError::new(vec![0, 1, 3], value),
                ))
                .into(),
            )),
        }
    }
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for StorageKeyRequirements {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for StorageKeyRequirements {}


#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;

    fn each_variant() -> [StorageKeyRequirements; 3] {
        [
            StorageKeyRequirements::Unique,
            StorageKeyRequirements::Multiple,
            StorageKeyRequirements::MultipleReferenceToLatest,
        ]
    }

    #[test]
    fn json_round_trip_each_variant() {
        use crate::serialization::JsonConvertible;
        for original in each_variant() {
            let json = original.to_json().expect("to_json");
            let recovered = StorageKeyRequirements::from_json(json).expect("from_json");
            assert_eq!(original, recovered);
        }
    }

    #[test]
    fn value_round_trip_each_variant() {
        use crate::serialization::ValueConvertible;
        for original in each_variant() {
            let value = original.to_object().expect("to_object");
            let recovered = StorageKeyRequirements::from_object(value).expect("from_object");
            assert_eq!(original, recovered);
        }
    }
}
