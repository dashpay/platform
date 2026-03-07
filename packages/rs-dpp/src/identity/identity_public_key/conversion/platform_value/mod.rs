mod v0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::IdentityPublicKey;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
pub use v0::*;

impl IdentityPublicKeyPlatformValueConversionMethodsV0 for IdentityPublicKey {
    fn to_object(&self) -> Result<Value, ProtocolError> {
        match self {
            IdentityPublicKey::V0(key) => key.to_object(),
        }
    }

    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        match self {
            IdentityPublicKey::V0(key) => key.to_cleaned_object(),
        }
    }

    fn into_object(self) -> Result<Value, ProtocolError> {
        match self {
            IdentityPublicKey::V0(key) => key.into_object(),
        }
    }

    fn from_object(
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => IdentityPublicKeyV0::from_object(value, platform_version).map(Into::into),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::from_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
