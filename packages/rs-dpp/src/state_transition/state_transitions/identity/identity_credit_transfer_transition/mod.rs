mod action;
mod fields;
mod identity_signed;
#[cfg(feature = "json-object")]
mod json_conversion;
mod state_transition_like;
mod v0;
mod v0_action;
mod v0_methods;
#[cfg(feature = "platform-value")]
mod value_conversion;

pub use action::IdentityCreditTransferTransitionAction;

use crate::serialization_traits::PlatformDeserializable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::Signable;
use crate::state_transition::identity_credit_transfer_transition::fields::property_names::RECIPIENT_ID;
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;
use crate::{Convertible, ProtocolError};
use bincode::{config, Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersionedDeserialize, PlatformVersioned};
use serde::Serialize;

pub type IdentityCreditTransferTransitionLatest = IdentityCreditTransferTransitionV0;

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
pub enum IdentityCreditTransferTransition {
    #[versioned(0)]
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
