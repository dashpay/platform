use crate::identity::IdentityPublicKey;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0Signable;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::PlatformSignable;

use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod accessors;
mod fields;
#[cfg(feature = "json-conversion")]
mod json_conversion;
mod methods;
mod types;
pub mod v0;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(Debug, Encode, Decode, PlatformSignable, Clone, PartialEq, Eq, From)]
//here we want to indicate that IdentityPublicKeyInCreation can be transformed into IdentityPublicKeyInCreationSignable
#[platform_signable(derive_into)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
pub enum IdentityPublicKeyInCreation {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityPublicKeyInCreationV0),
}

impl IdentityPublicKeyInCreation {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => Ok(IdentityPublicKeyInCreationV0::default().into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKeyInCreation::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl From<&IdentityPublicKeyInCreation> for IdentityPublicKey {
    fn from(val: &IdentityPublicKeyInCreation) -> Self {
        match val {
            IdentityPublicKeyInCreation::V0(v0) => v0.into(),
        }
    }
}

impl From<IdentityPublicKeyInCreation> for IdentityPublicKey {
    fn from(val: IdentityPublicKeyInCreation) -> Self {
        match val {
            IdentityPublicKeyInCreation::V0(v0) => v0.into(),
        }
    }
}

impl From<IdentityPublicKey> for IdentityPublicKeyInCreation {
    fn from(val: IdentityPublicKey) -> Self {
        match val {
            IdentityPublicKey::V0(_) => {
                let v0: IdentityPublicKeyInCreationV0 = val.into();
                v0.into()
            }
        }
    }
}

impl From<&IdentityPublicKey> for IdentityPublicKeyInCreation {
    fn from(val: &IdentityPublicKey) -> Self {
        match val {
            IdentityPublicKey::V0(_) => {
                let v0: IdentityPublicKeyInCreationV0 = val.into();
                v0.into()
            }
        }
    }
}
