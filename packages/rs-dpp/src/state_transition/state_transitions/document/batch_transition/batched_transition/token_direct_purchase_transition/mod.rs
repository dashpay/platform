pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenDirectPurchaseTransitionV0;

/// Represents a versioned transition for direct token purchases.
///
/// This enum allows for forward-compatible support of different versions
/// of the `TokenDirectPurchaseTransition` structure. Each variant corresponds
/// to a specific version of the transition logic and structure.
///
/// This transition type is used when a user intends to directly purchase tokens
/// by specifying the desired amount and the maximum total price they are willing to pay.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenDirectPurchaseTransition {
    /// Version 0 of the token direct purchase transition.
    ///
    /// This version includes the base document transition, the number of tokens
    /// to purchase, and the maximum total price the user agrees to pay.
    /// If the price in the contract is lower than the agreed price, the lower
    /// price is used.
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenDirectPurchaseTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenDirectPurchaseTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenDirectPurchaseTransition {}

impl Default for TokenDirectPurchaseTransition {
    fn default() -> Self {
        TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0::default())
        // since only v0
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
    use crate::state_transition::batch_transition::batched_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    pub(crate) fn fixture() -> TokenDirectPurchaseTransition {
        TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 13,
                token_contract_position: 2,
                data_contract_id: Identifier::new([0xa1; 32]),
                token_id: Identifier::new([0xb2; 32]),
                using_group_info: None,
            }),
            token_count: 100,
            total_agreed_price: 999_000,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `tokenCount` / `totalAgreedPrice` come from `rename_all =
        // "camelCase"` on the v0 struct.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 13,
                        "$tokenContractPosition": 2,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "tokenCount": 100,
                    "totalAgreedPrice": 999_000,
            })
        );
        let recovered = TokenDirectPurchaseTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `100u64`/`999_000u64`: `TokenAmount` and `Credits` are `u64`
        // aliases. `13u64`/`2u16`: identity_contract_nonce is `u64`,
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

                    "tokenCount": 100u64,
                    "totalAgreedPrice": 999_000u64,
            })
        );
        let recovered = TokenDirectPurchaseTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
