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
pub enum DocumentUpdatePriceTransition {
    #[display("V0({})", "_0")]
    V0(DocumentUpdatePriceTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentUpdatePriceTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentUpdatePriceTransition {}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::DocumentUpdatePriceTransitionV0;
    use platform_value::Identifier;

    fn base_fixture() -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::new([0xc1; 32]),
            identity_contract_nonce: 11,
            document_type_name: "post".to_string(),
            data_contract_id: Identifier::new([0xd2; 32]),
        })
    }

    /// Non-default values per field so a per-property assertion would catch
    /// any silent zero-out / flip on round-trip.
    fn fixture() -> DocumentUpdatePriceTransition {
        DocumentUpdatePriceTransition::V0(DocumentUpdatePriceTransitionV0 {
            base: base_fixture(),
            revision: 6,
            price: 555_000,
        })
    }

    fn assert_v0_fields(t: &DocumentUpdatePriceTransition) {
        let DocumentUpdatePriceTransition::V0(rec) = t;
        let DocumentBaseTransition::V0(base) = &rec.base else {
            panic!("expected base V0");
        };
        assert_eq!(base.id, Identifier::new([0xc1; 32]), "base.id");
        assert_eq!(base.identity_contract_nonce, 11, "base.identity_contract_nonce");
        assert_eq!(base.document_type_name, "post", "base.document_type_name");
        assert_eq!(base.data_contract_id, Identifier::new([0xd2; 32]), "base.data_contract_id");
        assert_eq!(rec.revision, 6, "revision");
        assert_eq!(rec.price, 555_000, "price");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        let recovered = <DocumentUpdatePriceTransition as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = ValueConvertible::to_object(&original).expect("to_object");
        let recovered = <DocumentUpdatePriceTransition as ValueConvertible>::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
