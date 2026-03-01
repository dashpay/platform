use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;

pub mod accessors;
pub mod fields;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_fee_strategy;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::balances::credits::CREDITS_PER_DUFF;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::ASSET_UNLOCK_TX_SIZE;
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

/// Minimal core per byte. Must be a fibonacci number
pub const MIN_CORE_FEE_PER_BYTE: u32 = 1;

/// Minimal amount in credits (x1000) to avoid "dust" error in Core
pub const MIN_WITHDRAWAL_AMOUNT: u64 =
    (ASSET_UNLOCK_TX_SIZE as u64) * (MIN_CORE_FEE_PER_BYTE as u64) * CREDITS_PER_DUFF;

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
    serde(tag = "$formatVersion")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path(
    "dpp.state_transition_serialization_versions.address_credit_withdrawal_state_transition"
)]
pub enum AddressCreditWithdrawalTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "0"))]
    V0(AddressCreditWithdrawalTransitionV0),
}

impl AddressCreditWithdrawalTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .address_funds
            .credit_withdrawal
        {
            0 => Ok(AddressCreditWithdrawalTransition::V0(
                AddressCreditWithdrawalTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "AddressCreditWithdrawalTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl StateTransitionFieldTypes for AddressCreditWithdrawalTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![OUTPUT_SCRIPT]
    }
}
