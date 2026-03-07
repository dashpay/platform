#[cfg(any(feature = "fixtures-and-mocks", feature = "state-transition-serde-conversion"))]
use crate::serialization::ValueConvertible;
#[cfg(all(
    feature = "json-conversion",
    any(
        feature = "fixtures-and-mocks",
        feature = "state-transition-serde-conversion"
    )
))]
use crate::serialization::JsonConvertible;
use crate::tokens::info::v0::IdentityTokenInfoV0;
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
    all(
        feature = "json-conversion",
        any(
            feature = "fixtures-and-mocks",
            feature = "state-transition-serde-conversion"
        )
    ),
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
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[cfg_attr(
    any(
        feature = "fixtures-and-mocks",
        feature = "state-transition-serde-conversion"
    ),
    derive(serde::Serialize, serde::Deserialize, ValueConvertible),
    serde(tag = "$formatVersion")
)]
pub enum IdentityTokenInfo {
    #[cfg_attr(
        any(
            feature = "fixtures-and-mocks",
            feature = "state-transition-serde-conversion"
        ),
        serde(rename = "0")
    )]
    V0(IdentityTokenInfoV0),
}


impl IdentityTokenInfo {
    pub fn new(frozen: bool, platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .token_versions
            .identity_token_info_default_structure_version
        {
            0 => Ok(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen })),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityTokenInfo::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    any(
        feature = "fixtures-and-mocks",
        feature = "state-transition-serde-conversion"
    )
))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn identity_token_info_json_round_trip() {
        let info = IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true });

        let json = info.to_json().expect("to_json should succeed");

        // Verify the version tag
        assert_eq!(
            json["$formatVersion"].as_str().unwrap(),
            "0",
            "Version tag should be '0'"
        );

        // Verify the boolean field
        assert_eq!(json["frozen"].as_bool().unwrap(), true);

        // round-trip
        let restored = IdentityTokenInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(info, restored);
    }

    #[test]
    fn identity_token_info_unfrozen_json_round_trip() {
        let info = IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: false });

        let json = info.to_json().expect("to_json should succeed");
        let restored = IdentityTokenInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(info, restored);
    }
}
