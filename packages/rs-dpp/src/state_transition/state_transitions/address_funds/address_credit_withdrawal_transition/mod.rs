#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;

pub mod accessors;
pub mod fields;
#[cfg(feature = "json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_fee_strategy;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
#[cfg(feature = "value-conversion")]
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
#[cfg(feature = "serde-conversion")]
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
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path(
    "dpp.state_transition_serialization_versions.address_credit_withdrawal_state_transition"
)]
pub enum AddressCreditWithdrawalTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
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

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for AddressCreditWithdrawalTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl ValueConvertible for AddressCreditWithdrawalTransition {}

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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;

    fn fixture() -> AddressCreditWithdrawalTransition {
        AddressCreditWithdrawalTransition::V0(AddressCreditWithdrawalTransitionV0::default())
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = AddressCreditWithdrawalTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = AddressCreditWithdrawalTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
