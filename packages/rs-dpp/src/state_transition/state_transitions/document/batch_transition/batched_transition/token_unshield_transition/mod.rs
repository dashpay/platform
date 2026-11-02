pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenUnshieldTransitionV0;

/// Moves `amount` of a token out of the token's shielded pool into `recipient_id`'s identity
/// token balance.
///
/// The Orchard bundle spends pool notes (revealing their nullifiers) with a value balance of
/// `+amount`; any change goes back into the pool as new notes. The batch owner pays the credits
/// fee and need not be the note owner or the recipient: the note owner's spend authorization
/// binds the token, the batch owner, the recipient and the amount through the sighash extra
/// data (`token_unshield_extra_sighash_data`), so the bundle cannot be redirected.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenUnshieldTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenUnshieldTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenUnshieldTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenUnshieldTransition {}

impl Default for TokenUnshieldTransition {
    fn default() -> Self {
        TokenUnshieldTransition::V0(TokenUnshieldTransitionV0::default())
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

    pub(crate) fn fixture() -> TokenUnshieldTransition {
        TokenUnshieldTransition::V0(TokenUnshieldTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 16,
                token_contract_position: 1,
                data_contract_id: Identifier::new([0xa1; 32]),
                token_id: Identifier::new([0xb2; 32]),
                using_group_info: None,
            }),
            amount: 2_500,
            recipient_id: Identifier::new([0xc3; 32]),
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
        assert_eq!(json["amount"], 2_500);
        assert_eq!(
            json["recipientId"],
            serde_json::json!(Identifier::new([0xc3; 32]))
        );
        assert!(json["actions"].is_array());
        let recovered = TokenUnshieldTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenUnshieldTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
