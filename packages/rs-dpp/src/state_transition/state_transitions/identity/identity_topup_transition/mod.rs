pub mod accessors;
pub mod fields;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
pub mod proved;
mod state_transition_like;
pub mod v0;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use fields::*;

use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
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
    "dpp.state_transition_serialization_versions.identity_top_up_state_transition"
)]
pub enum IdentityTopUpTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "0"))]
    V0(IdentityTopUpTransitionV0),
}

impl IdentityTopUpTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityTopUpTransition::V0(
                IdentityTopUpTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityTopUpTransition::default_versioned".to_string(),
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
                "Unknown IdentityTopUpTransition version for minimal_asset_lock_value {v}"
            ))),
        }
    }
}

impl StateTransitionFieldTypes for IdentityTopUpTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[repr(u64)]
enum MinimalAssetLockValue {
    V0 = 1000,
}
