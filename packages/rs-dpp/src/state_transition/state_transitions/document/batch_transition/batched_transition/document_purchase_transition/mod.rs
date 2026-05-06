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
pub enum DocumentPurchaseTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DocumentPurchaseTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentPurchaseTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentPurchaseTransition {}

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
    use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::DocumentPurchaseTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    pub(crate) fn fixture() -> DocumentPurchaseTransition {
        DocumentPurchaseTransition::V0(DocumentPurchaseTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::new([0xc1; 32]),
                identity_contract_nonce: 11,
                document_type_name: "post".to_string(),
                data_contract_id: Identifier::new([0xd2; 32]),
            }),
            revision: 3,
            price: 999_000,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Doubly-tagged externally enum: outer `V0` for the variant; inner
        // `V0` for the flattened `base`. `$identityContractNonce`,
        // `$revision`, and `price` are `u64`; JSON erases the size.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$id": Identifier::new([0xc1; 32]),
                        "$identityContractNonce": 11,
                        "$type": "post",
                        "$dataContractId": Identifier::new([0xd2; 32]),

                    "$revision": 3,
                    "price": 999_000,
            })
        );
        let recovered = DocumentPurchaseTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `11u64`/`3u64`/`999_000u64`: `IdentityNonce`, `Revision`, and
        // `Credits` are all `u64` aliases — explicit suffixes lock in
        // `Value::U64`.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "$baseFormatVersion": "0",
                        "$id": Identifier::new([0xc1; 32]),
                        "$identityContractNonce": 11u64,
                        "$type": "post",
                        "$dataContractId": Identifier::new([0xd2; 32]),

                    "$revision": 3u64,
                    "price": 999_000u64,
            })
        );
        let recovered = DocumentPurchaseTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
