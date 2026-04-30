mod from_document;
pub mod v0;
pub mod v0_methods;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum DocumentPurchaseTransition {
    #[display("V0({})", "_0")]
    V0(DocumentPurchaseTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentPurchaseTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentPurchaseTransition {}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::DocumentPurchaseTransitionV0;
    use platform_value::{Identifier, Value};
    use std::collections::BTreeMap;

    fn base_fixture() -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::new([0xc1; 32]),
            identity_contract_nonce: 11,
            document_type_name: "post".to_string(),
            data_contract_id: Identifier::new([0xd2; 32]),
        })
    }

    fn data_fixture() -> BTreeMap<String, Value> {
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), Value::Text("alice".to_string()));
        data
    }

    fn fixture() -> DocumentPurchaseTransition {
        DocumentPurchaseTransition::V0(DocumentPurchaseTransitionV0 {
            base: base_fixture(),
            revision: 3,
            price: 999_000,
        })
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        let recovered = <DocumentPurchaseTransition as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = ValueConvertible::to_object(&original).expect("to_object");
        let recovered = <DocumentPurchaseTransition as ValueConvertible>::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
