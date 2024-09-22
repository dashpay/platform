use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;

pub mod accessors;
pub mod fields;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_like;
pub mod v0;
pub mod v1;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::balances::credits::CREDITS_PER_DUFF;
use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::state_transition::identity_credit_withdrawal_transition::v1::{
    IdentityCreditWithdrawalTransitionV1, IdentityCreditWithdrawalTransitionV1Signable,
};
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

pub type IdentityCreditWithdrawalTransitionLatest = IdentityCreditWithdrawalTransitionV1;

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
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "1"))]
    V1(IdentityCreditWithdrawalTransitionV1),
}

impl IdentityCreditWithdrawalTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityCreditWithdrawalTransition::V0(
                IdentityCreditWithdrawalTransitionV0::default(),
            )),
            1 => Ok(IdentityCreditWithdrawalTransition::V1(
                IdentityCreditWithdrawalTransitionV1::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditWithdrawalTransition::default_versioned".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
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
