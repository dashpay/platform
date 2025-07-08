mod v0;

use crate::identity::{Identity, IdentityV0};
use crate::serialization::ValueConvertible;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::TryFromPlatformVersioned;
pub use v0::IdentityPlatformValueConversionMethodsV0;

impl ValueConvertible<'_> for Identity {}

impl IdentityPlatformValueConversionMethodsV0<'_> for Identity {}

impl TryFromPlatformVersioned<Value> for Identity {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => {
                let identity_v0: IdentityV0 =
                    platform_value::from_value(value).map_err(ProtocolError::ValueError)?;
                Ok(identity_v0.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::try_from_owned_value".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl TryFromPlatformVersioned<&Value> for Identity {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => {
                let identity_v0: IdentityV0 =
                    platform_value::from_value(value.clone()).map_err(ProtocolError::ValueError)?;
                Ok(identity_v0.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::try_from_owned_value".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
