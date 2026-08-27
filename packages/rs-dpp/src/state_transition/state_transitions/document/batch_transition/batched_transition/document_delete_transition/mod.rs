mod from_document;
pub mod v0;
pub mod v0_methods;
pub mod v1;
pub mod v1_methods;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::*;
pub use v1::*;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum DocumentDeleteTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DocumentDeleteTransitionV0),
    /// The indexOnly delete: base plus the document's full property-value
    /// tuple (there is no primary-storage row to fetch values from). Only
    /// accepted for indexOnly document types — the ABCI structure gates
    /// pair each variant with its storage mode. Serialization bound is
    /// raised to 1 at PV14 (STATE_TRANSITION_SERIALIZATION_VERSIONS_V3).
    #[display("V1({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "1"))]
    V1(DocumentDeleteTransitionV1),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentDeleteTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentDeleteTransition {}

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
    use crate::state_transition::batch_transition::document_delete_transition::v0::DocumentDeleteTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    pub(crate) fn fixture() -> DocumentDeleteTransition {
        DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::new([0xc1; 32]),
                identity_contract_nonce: 9,
                document_type_name: "post".to_string(),
                data_contract_id: Identifier::new([0xd2; 32]),
            }),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Doubly-tagged externally enum: outer `V0` for
        // `DocumentDeleteTransition`, inner `V0` for the flattened
        // `base: DocumentBaseTransition`. `$identityContractNonce` is
        // `u64`; JSON erases the size — see Value-path assertion for the
        // sized variant.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                    "$id": Identifier::new([0xc1; 32]),
                    "$identityContractNonce": 9,
                    "$type": "post",
                    "$dataContractId": Identifier::new([0xd2; 32]),

            })
        );
        let recovered = DocumentDeleteTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `9u64`: `IdentityNonce` is a `u64` alias; explicit suffix locks
        // in `Value::U64`. `Identifier`s interpolate via `Serialize` ->
        // `Value::Identifier`.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                    "$id": Identifier::new([0xc1; 32]),
                    "$identityContractNonce": 9u64,
                    "$type": "post",
                    "$dataContractId": Identifier::new([0xd2; 32]),

            })
        );
        let recovered = DocumentDeleteTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
