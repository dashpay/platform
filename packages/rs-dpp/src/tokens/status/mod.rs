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
pub enum TokenStatus {
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
