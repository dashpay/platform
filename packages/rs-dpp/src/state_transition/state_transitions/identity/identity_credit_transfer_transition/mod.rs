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

pub use action::{IdentityCreditTransferTransitionAction};

use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersioned, PlatformVersioned};
use bincode::{config, Decode, Encode};
use crate::serialization_traits::PlatformDeserializable;
use crate::serialization_traits::PlatformSerializable;
use crate::{Convertible, ProtocolError};
use crate::serialization_traits::Signable;
use crate::state_transition::StateTransitionFieldTypes;
use fields::*;
use crate::state_transition::identity_credit_transfer_transition::fields::property_names::RECIPIENT_ID;
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;


pub type IdentityCreditTransferTransitionLatest = IdentityCreditTransferTransitionV0;

#[derive(Debug, Clone, PlatformDeserialize, PlatformSerialize, PlatformSerdeVersioned, PlatformSignable, PlatformVersioned, Encode, Decode, From, PartialEq)]
#[platform_error_type(ProtocolError)]
#[platform_version_path(state_transitions.identity_state_transition)]
pub enum IdentityCreditTransferTransition {
    V0(IdentityCreditTransferTransitionV0),
}

impl StateTransitionFieldTypes for IdentityCreditTransferTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID, RECIPIENT_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

