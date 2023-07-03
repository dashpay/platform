mod action;
mod v0;
mod v0_action;
mod fields;
mod state_transition_like;
mod v0_methods;
#[cfg(feature = "json-object")]
mod json_conversion;
#[cfg(feature = "platform-value")]
mod value_conversion;

use fields::*;

pub use action::IdentityTopUpTransitionAction;
use platform_serialization::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersioned, PlatformVersioned};
use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable, Signable};
use bincode::{config, Decode, Encode};
use derive_more::From;
use crate::state_transition::StateTransitionFieldTypes;
use crate::ProtocolError;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;

#[derive(Debug, Clone, PlatformDeserialize, PlatformSerialize, PlatformSerdeVersioned, PlatformSignable, PlatformVersioned, Encode, Decode, From, PartialEq)]
#[platform_error_type(ProtocolError)]
#[platform_version_path(state_transitions.identity_state_transition)]
pub enum IdentityTopUpTransition {
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
