mod action;
mod v0;
mod v0_action;
mod fields;
mod identity_signed;
mod state_transition_like;
mod v0_methods;
#[cfg(feature = "json-object")]
mod json_conversion;
#[cfg(feature = "platform-value")]
mod value_conversion;

use fields::*;

pub use action::IdentityUpdateTransitionAction;
use platform_serialization::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersioned, PlatformVersioned};
use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable, Signable};
use bincode::{config, Decode, Encode};
use derive_more::From;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;
use crate::ProtocolError;
use crate::state_transition::identity_update_transition::fields::property_names::ADD_PUBLIC_KEYS_SIGNATURE;

#[derive(Debug, Clone, PlatformDeserialize, PlatformSerialize, PlatformSerdeVersioned, PlatformSignable, PlatformVersioned, Encode, Decode, From, PartialEq)]
#[platform_error_type(ProtocolError)]
#[platform_version_path(state_transitions.identity_state_transition)]
pub enum IdentityUpdateTransition {
    V0(IdentityUpdateTransitionV0),
}

impl StateTransitionFieldTypes for IdentityUpdateTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![
            SIGNATURE,
            ADD_PUBLIC_KEYS_SIGNATURE,
        ]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![
            SIGNATURE,
            SIGNATURE_PUBLIC_KEY_ID,
            ADD_PUBLIC_KEYS_SIGNATURE,
        ]
    }

}