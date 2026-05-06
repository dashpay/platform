pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenSetPriceForDirectPurchaseTransitionV0;

/// Represents a versioned transition for setting or updating the price of a token
/// available for direct purchase.
///
/// This transition allows a token owner or controlling group to define or remove a pricing
/// schedule for direct purchases. Setting the price to `None` disables further purchases
/// of the token.
///
/// This transition type supports **group actions**, meaning it can require **multi-signature
/// (multisig) authorization**. In such cases, multiple identities must agree and sign
/// the transition for it to be considered valid and executable.
///
/// Versioning enables forward compatibility by allowing future enhancements or changes
/// without breaking existing clients.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenSetPriceForDirectPurchaseTransition {
    /// Version 0 of the token set price for direct purchase transition.
    ///
    /// This version includes:
    /// - A base document transition.
    /// - An optional pricing schedule: `Some(...)` to set the token's price, or `None` to make it non-purchasable.
    /// - An optional public note.
    ///
    /// Group actions with multisig are supported in this version,
    /// enabling shared control over token pricing among multiple authorized identities.
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenSetPriceForDirectPurchaseTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenSetPriceForDirectPurchaseTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenSetPriceForDirectPurchaseTransition {}

impl Default for TokenSetPriceForDirectPurchaseTransition {
    fn default() -> Self {
        TokenSetPriceForDirectPurchaseTransition::V0(
            TokenSetPriceForDirectPurchaseTransitionV0::default(),
        ) // since only v0
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
    use crate::state_transition::batch_transition::batched_transition::token_set_price_for_direct_purchase_transition::v0::TokenSetPriceForDirectPurchaseTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip. `price: None` here exercises
    /// the "clear price (no longer purchasable)" wire shape; nested
    /// `TokenPricingSchedule` shapes are covered by that type's own tests.
    pub(crate) fn fixture() -> TokenSetPriceForDirectPurchaseTransition {
        TokenSetPriceForDirectPurchaseTransition::V0(TokenSetPriceForDirectPurchaseTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 13,
                token_contract_position: 2,
                data_contract_id: Identifier::new([0xa1; 32]),
                token_id: Identifier::new([0xb2; 32]),
                using_group_info: None,
            }),
            price: None,
            public_note: Some("clear".to_string()),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Doubly-tagged externally enum: outer `V0` for the variant; inner
        // `V0` for the flattened token base. The v0 struct has a stale
        // `serde(rename = "issuedToIdentityId")` on the `price` field
        // (copy-paste from the mint transition); that rename is the actual
        // wire key for `price` and round-trips correctly. `Option<...>::None`
        // serializes as `null`. `publicNote` comes from the camelCase rule.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 13,
                        "$tokenContractPosition": 2,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "issuedToIdentityId": serde_json::Value::Null,
                    "publicNote": "clear",
            })
        );
        let recovered =
            TokenSetPriceForDirectPurchaseTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `13u64`/`2u16`: identity_contract_nonce is `u64`,
        // token_contract_position is `u16`. `Value::Null` for the `None`
        // price.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 13u64,
                        "$tokenContractPosition": 2u16,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "issuedToIdentityId": platform_value::Value::Null,
                    "publicNote": "clear",
            })
        );
        let recovered =
            TokenSetPriceForDirectPurchaseTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
