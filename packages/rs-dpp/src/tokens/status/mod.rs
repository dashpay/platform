#[cfg(feature = "state-transition-serde-conversion")]
use crate::serialization::ValueConvertible;
#[cfg(all(feature = "json-conversion", feature = "state-transition-serde-conversion"))]
use crate::serialization::JsonConvertible;
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
    all(feature = "json-conversion", feature = "state-transition-serde-conversion"),
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
    feature = "state-transition-serde-conversion",
    derive(serde::Serialize, serde::Deserialize, ValueConvertible),
    serde(tag = "$formatVersion")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub enum TokenStatus {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "0"))]
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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "state-transition-serde-conversion"
))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn token_status_json_round_trip() {
        let status = TokenStatus::V0(TokenStatusV0 { paused: true });

        let json = status.to_json().expect("to_json should succeed");

        assert_eq!(json["$formatVersion"].as_str().unwrap(), "0");
        assert_eq!(json["paused"].as_bool().unwrap(), true);

        let restored = TokenStatus::from_json(json).expect("from_json should succeed");
        assert_eq!(status, restored);
    }
}
