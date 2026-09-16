pub mod accessors;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
use crate::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

pub type TokenPurchaseFromShieldedPoolTransitionLatest = TokenPurchaseFromShieldedPoolTransitionV0;

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

/// Token purchase paid from the credit pool.
///
/// Tokens bought at the token's direct purchase price and minted straight into the token's shielded pool, paid out of the credit shielded pool: an outputs-only Orchard bundle in the token pool (value balance minus `token_count`) and a spend bundle in the credit pool whose value balance is the price plus the fee. The price goes to the contract owner; no identity signs.
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
    "dpp.state_transition_serialization_versions.token_purchase_from_shielded_pool_state_transition"
)]
pub enum TokenPurchaseFromShieldedPoolTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenPurchaseFromShieldedPoolTransitionV0),
}

impl OptionallyAssetLockProved for TokenPurchaseFromShieldedPoolTransition {}

impl StateTransitionFieldTypes for TokenPurchaseFromShieldedPoolTransition {
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

    pub(crate) fn fixture() -> TokenPurchaseFromShieldedPoolTransition {
        TokenPurchaseFromShieldedPoolTransition::V0(TokenPurchaseFromShieldedPoolTransitionV0 {
            data_contract_id: Identifier::new([0x10; 32]),
            token_contract_position: 1,
            token_id: Identifier::new([0x12; 32]),
            token_count: 1003,
            total_agreed_price: 250004,
            token_actions: vec![fixture_action()],
            token_anchor: [0x76; 32],
            token_proof: vec![0x87; 192],
            token_binding_signature: [0x98; 64],
            fee_actions: vec![fixture_action()],
            fee_anchor: [0x7a; 32],
            fee_proof: vec![0x8b; 192],
            fee_binding_signature: [0x9c; 64],
            credit_amount: 250013,
        })
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
            TokenPurchaseFromShieldedPoolTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered =
            TokenPurchaseFromShieldedPoolTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
