pub mod v0;
mod v0_methods;
pub mod v1;
pub mod validate_structure;

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
use crate::serialization::JsonConvertible;
#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
use crate::serialization::ValueConvertible;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenMintTransitionV0;
pub use v1::TokenMintTransitionV1;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenMintTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenMintTransitionV0),
    /// Can issue a random amount (protocol version 15)
    #[display("V1({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "1"))]
    V1(TokenMintTransitionV1),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for TokenMintTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl ValueConvertible for TokenMintTransition {}

impl Default for TokenMintTransition {
    fn default() -> Self {
        // Version 0, which every protocol version accepts
        TokenMintTransition::V0(TokenMintTransitionV0::default())
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
    use crate::state_transition::batch_transition::batched_transition::token_mint_transition::v0::TokenMintTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    pub(crate) fn fixture() -> TokenMintTransition {
        TokenMintTransition::V0(TokenMintTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 13,
                token_contract_position: 2,
                data_contract_id: Identifier::new([0xa1; 32]),
                token_id: Identifier::new([0xb2; 32]),
                using_group_info: None,
            }),
            issued_to_identity_id: Some(Identifier::new([0xc3; 32])),
            amount: 5_000,
            public_note: Some("minting".to_string()),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Doubly-tagged externally enum: outer `V0` for the variant; inner
        // `V0` for the flattened token base. `issuedToIdentityId` is the
        // explicit serde rename on `issued_to_identity_id`; `amount`/`publicNote`
        // come from `rename_all = "camelCase"`. `amount` is `u64`; JSON
        // erases the size — see Value-path assertion below.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 13,
                        "$tokenContractPosition": 2,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "issuedToIdentityId": Identifier::new([0xc3; 32]),
                    "amount": 5_000,
                    "publicNote": "minting",
            })
        );
        let recovered = TokenMintTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `13u64`/`2u16`/`5_000u64`: explicit suffixes lock in the sized
        // variants (`Value::U64` / `Value::U16`) that JSON would erase.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 13u64,
                        "$tokenContractPosition": 2u16,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "issuedToIdentityId": Identifier::new([0xc3; 32]),
                    "amount": 5_000u64,
                    "publicNote": "minting",
            })
        );
        let recovered = TokenMintTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    /// A version 1 mint issuing a random amount from 1_000 to 5_000.
    pub(crate) fn fixture_v1() -> TokenMintTransition {
        TokenMintTransition::V1(TokenMintTransitionV1 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 13,
                token_contract_position: 2,
                data_contract_id: Identifier::new([0xa1; 32]),
                token_id: Identifier::new([0xb2; 32]),
                using_group_info: None,
            }),
            issued_to_identity_id: Some(Identifier::new([0xc3; 32])),
            amount: 5_000,
            public_note: Some("minting".to_string()),
            random_minimum_amount: Some(1_000),
        })
    }

    #[test]
    fn should_round_trip_a_version_1_mint_through_json() {
        use crate::serialization::JsonConvertible;
        let original = fixture_v1();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "1",
                "$baseFormatVersion": "0",
                "$identity-contract-nonce": 13,
                "$tokenContractPosition": 2,
                "$dataContractId": Identifier::new([0xa1; 32]),
                "$tokenId": Identifier::new([0xb2; 32]),
                "issuedToIdentityId": Identifier::new([0xc3; 32]),
                "amount": 5_000,
                "publicNote": "minting",
                "$randomMinimumAmount": 1_000,
            })
        );
        let recovered = TokenMintTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn should_round_trip_a_version_1_mint_through_a_value() {
        use crate::serialization::ValueConvertible;
        let original = fixture_v1();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "1",
                "$baseFormatVersion": "0",
                "$identity-contract-nonce": 13u64,
                "$tokenContractPosition": 2u16,
                "$dataContractId": Identifier::new([0xa1; 32]),
                "$tokenId": Identifier::new([0xb2; 32]),
                "issuedToIdentityId": Identifier::new([0xc3; 32]),
                "amount": 5_000u64,
                "publicNote": "minting",
                "$randomMinimumAmount": 1_000u64,
            })
        );
        let recovered = TokenMintTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
