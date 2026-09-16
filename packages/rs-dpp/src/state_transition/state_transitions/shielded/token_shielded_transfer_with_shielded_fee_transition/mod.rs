pub mod accessors;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::v0::TokenShieldedTransferWithShieldedFeeTransitionV0;
use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::v0::TokenShieldedTransferWithShieldedFeeTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

pub type TokenShieldedTransferWithShieldedFeeTransitionLatest =
    TokenShieldedTransferWithShieldedFeeTransitionV0;

use crate::identity::state_transition::OptionallyAssetLockProved;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Token shielded transfer paid from the credit pool.
///
/// A transfer inside a token's shielded pool whose fee is paid out of the credit shielded pool: an Orchard spend bundle in the token pool (value balance zero) and one in the credit pool (value balance the fee), both authorized by spend keys. No identity signs or is named anywhere.
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.token_shielded_transfer_with_shielded_fee_state_transition"
)]
pub enum TokenShieldedTransferWithShieldedFeeTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenShieldedTransferWithShieldedFeeTransitionV0),
}

impl OptionallyAssetLockProved for TokenShieldedTransferWithShieldedFeeTransition {}

impl StateTransitionFieldTypes for TokenShieldedTransferWithShieldedFeeTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::shielded::SerializedAction;
    use platform_value::Identifier;

    fn fixture_action() -> SerializedAction {
        SerializedAction {
            nullifier: [0x11; 32],
            rk: [0x22; 32],
            cmx: [0x33; 32],
            encrypted_note: vec![0x44; 216],
            cv_net: [0x55; 32],
            spend_auth_sig: [0x66; 64],
        }
    }

    pub(crate) fn fixture() -> TokenShieldedTransferWithShieldedFeeTransition {
        TokenShieldedTransferWithShieldedFeeTransition::V0(
            TokenShieldedTransferWithShieldedFeeTransitionV0 {
                data_contract_id: Identifier::new([0x10; 32]),
                token_contract_position: 1,
                token_id: Identifier::new([0x12; 32]),
                token_actions: vec![fixture_action()],
                token_anchor: [0x74; 32],
                token_proof: vec![0x85; 192],
                token_binding_signature: [0x96; 64],
                fee_actions: vec![fixture_action()],
                fee_anchor: [0x78; 32],
                fee_proof: vec![0x89; 192],
                fee_binding_signature: [0x9a; 64],
                credit_amount: 250011,
            },
        )
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
        assert!(json.get("tokenActions").is_some());
        assert!(json.get("feeActions").is_some());
        let recovered =
            TokenShieldedTransferWithShieldedFeeTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenShieldedTransferWithShieldedFeeTransition::from_object(value)
            .expect("from_object");
        assert_eq!(original, recovered);
    }
}
