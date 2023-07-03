mod v0;
mod v0_action;
mod action;
mod fields;
#[cfg(feature = "json-object")]
mod json_conversion;
#[cfg(feature = "platform-value")]
mod value_conversion;
mod state_transition_like;
mod v0_methods;
mod serialize;

pub use action::{IdentityCreateTransitionAction};
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use platform_versioning::{PlatformSerdeVersioned, PlatformVersioned};
use bincode::{config, Decode, Encode};
use crate::serialization_traits::PlatformDeserializable;
use crate::serialization_traits::PlatformSerializable;
use crate::{Convertible, ProtocolError};


pub type IdentityCreateTransitionLatest = IdentityCreateTransitionV0;

#[derive(Debug, Clone, PlatformDeserialize, PlatformSerialize, PlatformSerdeVersioned, PlatformVersioned, Encode, Decode, From, PartialEq)]
#[platform_error_type(ProtocolError)]
#[platform_version_path(state_transitions.identity_state_transition)]
pub enum IdentityCreateTransition {
    V0(IdentityCreateTransitionV0),
}
