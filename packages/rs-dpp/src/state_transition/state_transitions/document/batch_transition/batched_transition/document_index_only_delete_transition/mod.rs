mod from_document;
pub mod v0;
pub mod v0_methods;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::*;

/// The **indexOnly** delete: a distinct transition kind, not a version of
/// [`DocumentDeleteTransition`](super::DocumentDeleteTransition).
///
/// An indexOnly document has no primary-storage row, so a delete cannot
/// address anything by id — it carries the document's full property-value
/// tuple, from which (plus the signer as owner) every index entry is
/// recomputed and removed. Delete-by-id and delete-by-values differ in
/// payload, authorization model (fetch-then-check-ownership vs
/// self-authorizing values) and validation pipeline, which is exactly the
/// distinction the repo models as separate `DocumentTransition` kinds.
/// The ABCI structure gates pair each kind with its storage mode.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum DocumentIndexOnlyDeleteTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DocumentIndexOnlyDeleteTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentIndexOnlyDeleteTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentIndexOnlyDeleteTransition {}

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
    use platform_value::{platform_value, Identifier, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip. Carries ordinary document
    /// properties AND `$createdAt` under its system key — the payload shape
    /// of a delete on a type whose indexes use `$createdAt`.
    pub(crate) fn fixture() -> DocumentIndexOnlyDeleteTransition {
        DocumentIndexOnlyDeleteTransition::V0(DocumentIndexOnlyDeleteTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::new([0xc1; 32]),
                identity_contract_nonce: 9,
                document_type_name: "like".to_string(),
                data_contract_id: Identifier::new([0xd2; 32]),
            }),
            // Text and integer property values only: JSON has no
            // `Identifier` type, so an identifier-valued property would
            // come back as its base58 text — same restriction the create
            // transition's round-trip fixture observes.
            data: BTreeMap::from([
                ("$createdAt".to_string(), Value::U64(1_700_000_000_123)),
                ("hashtag".to_string(), Value::Text("dash".to_string())),
                ("weight".to_string(), Value::U64(3)),
            ]),
        })
    }

    /// The manual `Deserialize` peels `BASE_FIELD_NAMES` off the flat
    /// object and routes everything else into `data` — an unlisted base
    /// key would silently land in document data, so these full-wire-shape
    /// assertions are the guard on that list staying in sync with the
    /// base transition's fields.
    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Outer `$formatVersion` for `DocumentIndexOnlyDeleteTransition`,
        // inner `$baseFormatVersion` for the flattened base, and both
        // flattened payload maps merged at the top level: the base's `$`
        // system keys next to the document's own properties (with
        // `$createdAt` riding in `data` under its system key).
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                "$id": Identifier::new([0xc1; 32]),
                "$identityContractNonce": 9,
                "$type": "like",
                "$dataContractId": Identifier::new([0xd2; 32]),
                "$createdAt": 1_700_000_000_123u64,
                "hashtag": "dash",
                "weight": 3,
            })
        );
        let recovered = DocumentIndexOnlyDeleteTransition::from_json(json).expect("from_json");
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
                "$id": Identifier::new([0xc1; 32]),
                "$identityContractNonce": 9u64,
                "$type": "like",
                "$dataContractId": Identifier::new([0xd2; 32]),
                "$createdAt": 1_700_000_000_123u64,
                "hashtag": "dash",
                "weight": 3u64,
            })
        );
        let recovered = DocumentIndexOnlyDeleteTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
