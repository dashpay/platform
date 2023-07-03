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

use crate::serialization_traits::PlatformDeserializable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::Signable;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;
use crate::{Convertible, ProtocolError};
pub use action::IdentityCreateTransitionAction;
use bincode::{config, Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersioned, PlatformVersioned};

pub type IdentityCreateTransitionLatest = IdentityCreateTransitionV0;

#[derive(
    Debug,
    Clone,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSerdeVersioned,
    PlatformSignable,
    PlatformVersioned,
    Encode,
    Decode,
    From,
    PartialEq,
)]
#[platform_error_type(ProtocolError)]
#[platform_version_path(state_transitions.identity_state_transition)]
pub enum IdentityCreateTransition {
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
