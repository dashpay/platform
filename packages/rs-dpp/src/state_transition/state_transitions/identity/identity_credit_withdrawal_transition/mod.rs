use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;

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

pub use action::IdentityCreditWithdrawalTransitionAction;

use crate::contracts::withdrawals_contract::property_names::OUTPUT_SCRIPT;
use crate::serialization_traits::PlatformDeserializable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::Signable;
use crate::state_transition::StateTransitionFieldTypes;
use crate::{Convertible, ProtocolError};
use bincode::{config, Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersioned, PlatformVersioned};

pub type IdentityCreditWithdrawalTransitionLatest = IdentityCreditWithdrawalTransitionV0;

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
pub enum IdentityCreditWithdrawalTransition {
    V0(IdentityCreditWithdrawalTransitionV0),
}

impl StateTransitionFieldTypes for IdentityCreditWithdrawalTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, OUTPUT_SCRIPT]
    }
}
