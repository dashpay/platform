pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenConfigUpdateTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum TokenConfigUpdateTransition {
    #[display("V0({})", "_0")]
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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::batched_transition::token_config_update_transition::v0::TokenConfigUpdateTransitionV0;
    use platform_value::Identifier;

    fn token_base_fixture() -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 13,
            token_contract_position: 2,
            data_contract_id: Identifier::new([0xa1; 32]),
            token_id: Identifier::new([0xb2; 32]),
            using_group_info: None,
        })
    }

    /// Non-default values per field so a per-property assertion would catch
    /// any silent zero-out / flip on round-trip.
    fn fixture() -> TokenConfigUpdateTransition {
        TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
            base: token_base_fixture(),
            update_token_configuration_item: TokenConfigurationChangeItem::TokenConfigurationNoChange,
            public_note: Some("config update".to_string()),
        })
    }

    fn assert_v0_fields(t: &TokenConfigUpdateTransition) {
        let TokenConfigUpdateTransition::V0(rec) = t;
        let TokenBaseTransition::V0(base) = &rec.base;
        assert_eq!(base.identity_contract_nonce, 13, "base.identity_contract_nonce");
        assert_eq!(base.token_contract_position, 2, "base.token_contract_position");
        assert_eq!(base.data_contract_id, Identifier::new([0xa1; 32]), "base.data_contract_id");
        assert_eq!(base.token_id, Identifier::new([0xb2; 32]), "base.token_id");
        assert_eq!(base.using_group_info, None, "base.using_group_info");
        assert_eq!(
            rec.update_token_configuration_item,
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
            "update_token_configuration_item"
        );
        assert_eq!(rec.public_note, Some("config update".to_string()), "public_note");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        let recovered = <TokenConfigUpdateTransition as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = ValueConvertible::to_object(&original).expect("to_object");
        let recovered = <TokenConfigUpdateTransition as ValueConvertible>::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
