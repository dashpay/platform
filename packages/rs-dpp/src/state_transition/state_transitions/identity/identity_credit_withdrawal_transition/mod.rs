use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;

pub mod accessors;
mod fields;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_like;
pub mod v0;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::serialization::Signable;
use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;
use crate::{Convertible, ProtocolError};
use crate::version::PlatformVersionCurrentVersion;
use bincode::{config, Decode, Encode};
use data_contracts::withdrawals_contract::document_types::withdrawal::properties::OUTPUT_SCRIPT;
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersionedDeserialize, PlatformVersioned};
use crate::state_transition::StateTransitionValueConvert;
use serde::Serialize;

pub type IdentityCreditWithdrawalTransitionLatest = IdentityCreditWithdrawalTransitionV0;

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
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
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.identity_credit_withdrawal_state_transition"
)]
pub enum IdentityCreditWithdrawalTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", versioned(0))]
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
