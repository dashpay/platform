use bincode::Encode;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::{errors::ProtocolError, prelude::TimestampMillis, util::deserializer::ProtocolVersion};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    #[serde(default)]
    pub block_height: u64,
    #[serde(default)]
    pub core_chain_locked_height: u64,
    #[serde(default)]
    pub time_ms: TimestampMillis,
    #[serde(default)]
    pub protocol_version: ProtocolVersion,
}

impl std::convert::TryFrom<&str> for Metadata {
    type Error = ProtocolError;

    fn try_from(d: &str) -> Result<Metadata, Self::Error> {
        serde_json::from_str(d).map_err(|e| ProtocolError::EncodingError(e.to_string()))
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for Metadata {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for Metadata {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::serialization::{JsonConvertible, ValueConvertible};
    use platform_value::platform_value;
    use serde_json::json;

    /// Non-default values per field. `Metadata` crosses to JS as the
    /// `$metadata` field of `ExtendedDocument`, so its u64 fields go through
    /// `json_safe_fields` (numbers below 2^53, strings above).
    fn fixture() -> Metadata {
        Metadata {
            block_height: 1_234_567,
            core_chain_locked_height: 2_222_333,
            time_ms: 1_700_000_000_000,
            protocol_version: 9,
        }
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int info erased in JSON; the value-path assertion locks the
        // typed variants (u64 heights/time, u32 protocolVersion).
        assert_eq!(
            json,
            json!({
                "blockHeight": 1_234_567,
                "coreChainLockedHeight": 2_222_333,
                "timeMs": 1_700_000_000_000u64,
                "protocolVersion": 9,
            })
        );
        let recovered = Metadata::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "blockHeight": 1_234_567u64,
                "coreChainLockedHeight": 2_222_333u64,
                "timeMs": 1_700_000_000_000u64,
                "protocolVersion": 9u32,
            })
        );
        let recovered = Metadata::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    /// Values above JS `Number.MAX_SAFE_INTEGER` must stringify in JSON
    /// (json_safe_fields contract) and still round-trip.
    #[test]
    fn json_stringifies_unsafe_u64() {
        let original = Metadata {
            block_height: u64::MAX,
            ..fixture()
        };
        let json = original.to_json().expect("to_json");
        assert_eq!(json["blockHeight"], json!(u64::MAX.to_string()));
        let recovered = Metadata::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }
}
