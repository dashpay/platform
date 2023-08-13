pub mod accessors;
mod fields;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_like;
pub mod v0;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::serialization::Signable;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;
use crate::version::PlatformVersionCurrentVersion;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersionedDeserialize, PlatformVersioned};
use serde::Serialize;

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
    derive(Serialize, PlatformSerdeVersionedDeserialize),
    serde(untagged)
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.identity_create_state_transition"
)]
pub enum IdentityCreateTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", versioned(0))]
    V0(IdentityCreateTransitionV0),
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
