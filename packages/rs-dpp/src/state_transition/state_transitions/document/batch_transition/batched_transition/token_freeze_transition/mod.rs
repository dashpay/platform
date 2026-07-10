pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenFreezeTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenFreezeTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenFreezeTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenFreezeTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenFreezeTransition {}

impl Default for TokenFreezeTransition {
    fn default() -> Self {
        TokenFreezeTransition::V0(TokenFreezeTransitionV0::default()) // since only v0
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
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::batched_transition::token_freeze_transition::v0::TokenFreezeTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    pub(crate) fn fixture() -> TokenFreezeTransition {
        TokenFreezeTransition::V0(TokenFreezeTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 13,
                token_contract_position: 2,
                data_contract_id: Identifier::new([0xa1; 32]),
                token_id: Identifier::new([0xb2; 32]),
                using_group_info: None,
            }),
            identity_to_freeze_id: Identifier::new([0xc3; 32]),
            public_note: Some("freeze".to_string()),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Doubly-tagged externally enum: outer `V0` for the variant; inner
        // `V0` for the flattened token base. `frozenIdentityId` is the
        // explicit serde rename on `identity_to_freeze_id`. `publicNote` is
        // produced by `rename_all = "camelCase"`.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 13,
                        "$tokenContractPosition": 2,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "frozenIdentityId": Identifier::new([0xc3; 32]),
                    "publicNote": "freeze",
            })
        );
        let recovered = TokenFreezeTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `13u64`/`2u16`: identity_contract_nonce is `u64`,
        // token_contract_position is `u16`.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 13u64,
                        "$tokenContractPosition": 2u16,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "frozenIdentityId": Identifier::new([0xc3; 32]),
                    "publicNote": "freeze",
            })
        );
        let recovered = TokenFreezeTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
