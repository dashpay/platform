mod v0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::IdentityPublicKey;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::Value as JsonValue;
use v0::IdentityPublicKeyJsonConversionMethodsV0;
pub use v0::*;

impl IdentityPublicKeyJsonConversionMethodsV0 for IdentityPublicKey {
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            IdentityPublicKey::V0(key) => key.to_json(),
        }
    }

    fn to_json_object(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            IdentityPublicKey::V0(key) => key.to_json_object(),
        }
    }

    fn from_json_object(
        raw_object: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => {
                IdentityPublicKeyV0::from_json_object(raw_object, platform_version).map(Into::into)
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::from_json_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
