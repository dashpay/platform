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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;

    // `StorageKeyRequirements` is `#[repr(u8)]` with `serde_repr` —
    // it serializes as the bare numeric discriminant (not a struct/string).
    // JSON erases the u8 distinction; the value-path tests use `0u8`/`1u8`/`2u8`.

    #[test]
    fn json_round_trip_unique() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = StorageKeyRequirements::Unique;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!(0));
        let recovered = StorageKeyRequirements::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_multiple() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = StorageKeyRequirements::Multiple;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!(1));
        let recovered = StorageKeyRequirements::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_multiple_reference_to_latest() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = StorageKeyRequirements::MultipleReferenceToLatest;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!(2));
        let recovered = StorageKeyRequirements::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_unique() {
        use crate::serialization::ValueConvertible;
        use platform_value::Value;
        let original = StorageKeyRequirements::Unique;
        let value = original.to_object().expect("to_object");
        // `0u8`: `#[repr(u8)]` with `Serialize_repr` produces `Value::U8(0)`.
        assert_eq!(value, Value::U8(0));
        let recovered = StorageKeyRequirements::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_multiple() {
        use crate::serialization::ValueConvertible;
        use platform_value::Value;
        let original = StorageKeyRequirements::Multiple;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, Value::U8(1));
        let recovered = StorageKeyRequirements::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_multiple_reference_to_latest() {
        use crate::serialization::ValueConvertible;
        use platform_value::Value;
        let original = StorageKeyRequirements::MultipleReferenceToLatest;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, Value::U8(2));
        let recovered = StorageKeyRequirements::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
