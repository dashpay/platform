mod action;
mod fields;
#[cfg(feature = "json-object")]
mod json_conversion;
mod state_transition_like;
mod v0;
mod v0_action;
mod v0_methods;
#[cfg(feature = "platform-value")]
mod value_conversion;

use fields::*;

use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable, Signable};
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;
use crate::ProtocolError;
pub use action::IdentityTopUpTransitionAction;
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersionedDeserialize, PlatformVersioned};
use serde::Serialize;

#[derive(
    Debug,
    Clone,
    Serialize,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSerdeVersionedDeserialize,
    PlatformSignable,
    PlatformVersioned,
    Encode,
    Decode,
    From,
    PartialEq,
)]
#[platform_error_type(ProtocolError)]
#[platform_version_path(state_transitions.identity_state_transition)]
#[serde(untagged)]
pub enum IdentityTopUpTransition {
    #[versioned(0)]
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
