#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::tokens::status::v0::TokenStatusV0;
use crate::ProtocolError;
use bincode::Encode;
use derive_more::From;
use platform_serialization::de::Decode;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
mod methods;
pub mod v0;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub enum TokenStatus {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenStatusV0),
}

impl TokenStatus {
    pub fn new(paused: bool, platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .token_versions
            .identity_token_status_default_structure_version
        {
            0 => Ok(TokenStatus::V0(TokenStatusV0 { paused })),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityTokenStatus::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn token_status_json_round_trip() {
        let status = TokenStatus::V0(TokenStatusV0 { paused: true });

        let json = status.to_json().expect("to_json should succeed");

        assert_eq!(json["$formatVersion"].as_str().unwrap(), "0");
        assert!(json["paused"].as_bool().unwrap());

        let restored = TokenStatus::from_json(json).expect("from_json should succeed");
        assert_eq!(status, restored);
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_tokenstatus {
    use super::*;
    use crate::tokens::status::v0::TokenStatusV0;
    use platform_value::platform_value;
    use serde_json::json;

    fn fixture() -> TokenStatus {
        TokenStatus::V0(TokenStatusV0 { paused: true })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Internally-tagged enum (`tag = "$formatVersion"`); `TokenStatusV0` has
        // `rename_all = "camelCase"` but the only field (`paused`) is already
        // a single-token name, so the wire key matches the source identifier.
        assert_eq!(json, json!({"$formatVersion": "0", "paused": true}));
        let recovered = TokenStatus::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({"$formatVersion": "0", "paused": true})
        );
        let recovered = TokenStatus::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
