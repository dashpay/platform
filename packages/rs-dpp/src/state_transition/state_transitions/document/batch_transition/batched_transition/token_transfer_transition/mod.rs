pub mod v0;
pub mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenTransferTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenTransferTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenTransferTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenTransferTransition {}

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
    use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    pub(crate) fn fixture() -> TokenTransferTransition {
        TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 14,
                token_contract_position: 3,
                data_contract_id: Identifier::new([0xa1; 32]),
                token_id: Identifier::new([0xb2; 32]),
                using_group_info: None,
            }),
            amount: 250,
            recipient_id: Identifier::new([0xc3; 32]),
            public_note: Some("transfer note".to_string()),
            shared_encrypted_note: None,
            private_encrypted_note: None,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Doubly-tagged externally: outer `V0` for the variant; inner `V0`
        // for the flattened token base. `$amount` is `u64`; JSON erases the
        // size. Base fields are flattened into the outer object.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 14,
                        "$tokenContractPosition": 3,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "$amount": 250,
                    "recipientId": Identifier::new([0xc3; 32]),
                    "publicNote": "transfer note",
                    "sharedEncryptedNote": null,
                    "privateEncryptedNote": null,
            })
        );
        let recovered = TokenTransferTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `14u64`: `IdentityNonce` is `u64`. `3u16`: token_contract_position
        // is `u16`. `250u64`: amount is `u64`.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 14u64,
                        "$tokenContractPosition": 3u16,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "$amount": 250u64,
                    "recipientId": Identifier::new([0xc3; 32]),
                    "publicNote": "transfer note",
                    "sharedEncryptedNote": null,
                    "privateEncryptedNote": null,
            })
        );
        let recovered = TokenTransferTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
