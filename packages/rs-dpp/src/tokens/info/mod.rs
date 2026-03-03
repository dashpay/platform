#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use crate::serialization::ValueConvertible;
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
    derive(serde::Serialize, serde::Deserialize),
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

#[cfg(all(
    feature = "json-conversion",
    any(
        feature = "fixtures-and-mocks",
        feature = "state-transition-serde-conversion"
    )
))]
impl JsonConvertible for IdentityTokenInfo {}
#[cfg(any(
    feature = "fixtures-and-mocks",
    feature = "state-transition-serde-conversion"
))]
impl ValueConvertible for IdentityTokenInfo {}

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
