mod v0;

use crate::identity::{Identity, IdentityV0};
use crate::serialization::ValueConvertible;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
pub use v0::*;

impl ValueConvertible for Identity {}

impl IdentityPlatformValueConversionMethodsV0 for Identity {}

impl Identity {
    pub fn try_from_owned_value(
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
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

    pub fn try_from_borrowed_value(
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
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
