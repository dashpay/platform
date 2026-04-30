pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenUnfreezeTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum TokenUnfreezeTransition {
    #[display("V0({})", "_0")]
    V0(TokenUnfreezeTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenUnfreezeTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenUnfreezeTransition {}

impl Default for TokenUnfreezeTransition {
    fn default() -> Self {
        TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0::default()) // since only v0
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::batched_transition::token_unfreeze_transition::v0::TokenUnfreezeTransitionV0;
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

    fn fixture() -> TokenUnfreezeTransition {
        TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0 {
            base: token_base_fixture(),
            frozen_identity_id: Identifier::new([0xc3; 32]),
            public_note: Some("unfreeze".to_string()),
        })
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        let recovered = <TokenUnfreezeTransition as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = ValueConvertible::to_object(&original).expect("to_object");
        let recovered = <TokenUnfreezeTransition as ValueConvertible>::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
