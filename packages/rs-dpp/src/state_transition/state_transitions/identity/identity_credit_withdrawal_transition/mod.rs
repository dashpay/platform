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

use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use data_contracts::withdrawals_contract::v1::document_types::withdrawal::properties::OUTPUT_SCRIPT;
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::PlatformVersioned;
use serde::{Deserialize, Serialize};

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
    derive(Serialize, Deserialize),
    serde(tag = "$version")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path(
    "dpp.state_transition_serialization_versions.identity_credit_withdrawal_state_transition"
)]
pub enum IdentityCreditWithdrawalTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "0"))]
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

impl OptionallyAssetLockProved for IdentityCreditWithdrawalTransition {}
