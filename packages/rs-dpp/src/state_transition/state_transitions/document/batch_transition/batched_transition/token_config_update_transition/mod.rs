pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenConfigUpdateTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenConfigUpdateTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenConfigUpdateTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenConfigUpdateTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenConfigUpdateTransition {}

impl Default for TokenConfigUpdateTransition {
    fn default() -> Self {
        TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0::default())
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
    use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::batched_transition::token_config_update_transition::v0::TokenConfigUpdateTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip. The fixture uses the
    /// `TokenConfigurationNoChange` unit variant of
    /// `TokenConfigurationChangeItem` so the inline wire shape stays small;
    /// the richer variants are covered by `TokenConfigurationChangeItem`'s
    /// own tests.
    pub(crate) fn fixture() -> TokenConfigUpdateTransition {
        TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 13,
                token_contract_position: 2,
                data_contract_id: Identifier::new([0xa1; 32]),
                token_id: Identifier::new([0xb2; 32]),
                using_group_info: None,
            }),
            update_token_configuration_item:
                TokenConfigurationChangeItem::TokenConfigurationNoChange,
            public_note: Some("config update".to_string()),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `TokenConfigurationChangeItem` is now internally tagged, so its unit
        // variant `TokenConfigurationNoChange` serializes to the object
        // `{"$type":"tokenConfigurationNoChange"}` (not a bare string).
        // `updateTokenConfigurationItem` and `publicNote` come from the parent
        // struct's camelCase rule.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 13,
                        "$tokenContractPosition": 2,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "updateTokenConfigurationItem": { "$type": "tokenConfigurationNoChange" },
                    "publicNote": "config update",
            })
        );
        let recovered = TokenConfigUpdateTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `13u64`/`2u16`: identity_contract_nonce is `u64`,
        // token_contract_position is `u16`. The unit-variant
        // `TokenConfigurationNoChange` is now an internally-tagged map
        // `{"$type":"tokenConfigurationNoChange"}`, matching its JSON form.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$identity-contract-nonce": 13u64,
                        "$tokenContractPosition": 2u16,
                        "$dataContractId": Identifier::new([0xa1; 32]),
                        "$tokenId": Identifier::new([0xb2; 32]),

                    "updateTokenConfigurationItem": { "$type": "tokenConfigurationNoChange" },
                    "publicNote": "config update",
            })
        );
        let recovered = TokenConfigUpdateTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
