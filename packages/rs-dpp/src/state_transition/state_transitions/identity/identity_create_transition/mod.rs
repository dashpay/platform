mod v0;
mod v0_action;
mod action;
mod fields;

pub use action::{IdentityCreateTransitionAction};
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use platform_versioning::PlatformVersioned;
use bincode::{config, Decode, Encode};


pub type IdentityCreateTransitionLatest = IdentityCreateTransitionV0;

#[derive(Debug, Clone, PlatformDeserialize, PlatformSerialize, PlatformVersioned, Encode, Decode, From, PartialEq)]
#[platform_error_type(ProtocolError)]
#[platform_version_path(state_transitions.contract_create_state_transition)]
pub enum IdentityCreateTransition {
    V0(IdentityCreateTransitionV0),
}
