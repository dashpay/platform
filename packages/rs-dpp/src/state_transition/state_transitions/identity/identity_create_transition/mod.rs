pub mod accessors;
mod fields;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
pub mod proved;
mod state_transition_like;
pub mod v0;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
use serde::{Deserialize, Serialize};

pub type IdentityCreateTransitionLatest = IdentityCreateTransitionV0;

#[derive(
    Debug,
    Clone,
    Decode,
    Encode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$version")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.identity_create_state_transition"
)]
pub enum IdentityCreateTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "0"))]
    V0(IdentityCreateTransitionV0),
}

impl IdentityCreateTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityCreateTransition::V0(
                IdentityCreateTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreateTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn get_minimal_asset_lock_value(
        platform_version: &PlatformVersion,
    ) -> Result<u64, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .minimal_asset_lock_value
        {
            0 => Ok(MinimalAssetLockValue::V0 as u64),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreateTransition version for minimal_asset_lock_value {v}"
            ))),
        }
    }
}

impl StateTransitionFieldTypes for IdentityCreateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, PUBLIC_KEYS_SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[repr(u64)]
pub enum MinimalAssetLockValue {
    V0 = 120000,
}
