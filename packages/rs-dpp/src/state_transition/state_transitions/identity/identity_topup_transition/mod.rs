pub mod accessors;
mod fields;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_like;
pub(crate) mod v0;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use fields::*;

use crate::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersionedDeserialize, PlatformVersioned};
use serde::Serialize;

#[derive(
    Debug,
    Clone,
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
#[platform_version_path(
    "dpp.state_transition_serialization_versions.identity_top_up_state_transition"
)]
pub enum IdentityTopUpTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", versioned(0))]
    V0(IdentityTopUpTransitionV0),
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
