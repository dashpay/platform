mod from_document;
pub mod v0;
pub mod v0_methods;

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
pub enum DocumentEraseTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DocumentEraseTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentEraseTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentEraseTransition {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    pub(crate) fn fixture() -> DocumentEraseTransition {
        DocumentEraseTransition::V0(DocumentEraseTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::new([0xe3; 32]),
                identity_contract_nonce: 11,
                document_type_name: "note".to_string(),
                data_contract_id: Identifier::new([0xf4; 32]),
            }),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Doubly-tagged externally enum: outer `V0` for
        // `DocumentEraseTransition`, inner `V0` for the flattened
        // `base: DocumentBaseTransition`.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                "$id": Identifier::new([0xe3; 32]),
                "$identityContractNonce": 11,
                "$type": "note",
                "$dataContractId": Identifier::new([0xf4; 32]),
            })
        );
        let recovered = DocumentEraseTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                "$id": Identifier::new([0xe3; 32]),
                "$identityContractNonce": 11u64,
                "$type": "note",
                "$dataContractId": Identifier::new([0xf4; 32]),
            })
        );
        let recovered = DocumentEraseTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
