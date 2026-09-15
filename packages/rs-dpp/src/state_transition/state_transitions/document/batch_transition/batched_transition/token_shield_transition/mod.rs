pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenShieldTransitionV0;

/// Moves `amount` of a token from the batch owner's identity token balance into the token's
/// shielded pool.
///
/// The Orchard bundle is outputs-only (spends disabled) with a value balance of `-amount`: the
/// new notes are created for whoever the owner addressed them to, and nothing is spent from the
/// pool. The identity signature over the batch binds the bundle, the token and the amount, so
/// the bundle carries no extra sighash data of its own.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenShieldTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenShieldTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenShieldTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenShieldTransition {}

impl Default for TokenShieldTransition {
    fn default() -> Self {
        TokenShieldTransition::V0(TokenShieldTransitionV0::default())
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
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
    use platform_value::Identifier;

    pub(crate) fn fixture() -> TokenShieldTransition {
        TokenShieldTransition::V0(TokenShieldTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 15,
                token_contract_position: 1,
                data_contract_id: Identifier::new([0xa1; 32]),
                token_id: Identifier::new([0xb2; 32]),
                using_group_info: None,
            }),
            amount: 4_000,
            actions: vec![SerializedAction {
                nullifier: [0x11; 32],
                rk: [0x22; 32],
                cmx: [0x33; 32],
                encrypted_note: vec![0x44; 216],
                cv_net: [0x55; 32],
                spend_auth_sig: [0x66; 64],
            }],
            anchor: [0x77; 32],
            proof: vec![0x88; 8],
            binding_signature: [0x99; 64],
        })
    }

    #[test]
    fn json_round_trip_keeps_envelope_and_bundle() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
        assert_eq!(json["$baseFormatVersion"], "0");
        assert_eq!(json["$tokenContractPosition"], 1);
        assert_eq!(json["amount"], 4_000);
        assert!(json["actions"].is_array());
        assert!(json["anchor"].is_string(), "byte fields are base64 in JSON");
        let recovered = TokenShieldTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenShieldTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
