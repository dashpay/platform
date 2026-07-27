pub mod document_base_transition_trait;
mod fields;
mod from_document;
pub mod v0;
mod v0_methods;
pub mod v1;
mod v1_methods;

#[cfg(any(feature = "value-conversion", feature = "json-conversion"))]
use crate::data_contract::DataContract;
use crate::state_transition::batch_transition::document_base_transition::v0::{
    DocumentBaseTransitionV0, DocumentTransitionObjectLike,
};
use crate::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
#[cfg(any(feature = "value-conversion", feature = "json-conversion"))]
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::{Display, From};
pub use fields::*;
#[cfg(any(feature = "value-conversion", feature = "json-conversion"))]
use platform_value::Value;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "json-conversion")]
use serde_json::Value as JsonValue;
#[cfg(feature = "value-conversion")]
use std::collections::BTreeMap;

// Internal tagging with `$baseFormatVersion`. `DocumentBaseTransition` is
// `serde(flatten)`'d into every document leaf transition's V0 struct; the
// leaf wrappers use `tag = "$formatVersion"`, so a distinct key per nesting
// level prevents collision at the same flattened level. The flat-shape
// convention is preserved across every document and token transition, so
// every consumer reads `tx["$id"]`, `tx["$identityContractNonce"]`, etc. at
// the top level (no `"base"` envelope to traverse).
//
// `DocumentCreateTransitionV0` and `DocumentReplaceTransitionV0` have an
// extra constraint: they combine `serde(flatten) base` with `serde(flatten)
// data: BTreeMap<String, Value>` (a catchall) — under the auto-derived
// `Deserialize` the catchall would steal the base's discriminator and
// fields. Both leaves carry a manual `Deserialize` impl that explicitly
// routes `$baseFormatVersion` + the known base struct fields to `base`
// before letting the catchall claim what's left. See comments on
// those impls for detail.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$baseFormatVersion")
)]
pub enum DocumentBaseTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DocumentBaseTransitionV0),
    #[display("V1({})", "_1")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "1"))]
    V1(DocumentBaseTransitionV1),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentBaseTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentBaseTransition {}

impl Default for DocumentBaseTransition {
    fn default() -> Self {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0::default()) // since only v0
    }
}

impl DocumentTransitionObjectLike for DocumentBaseTransition {
    #[cfg(feature = "json-conversion")]
    fn from_json_object(
        json_str: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let value: Value = json_str.into();
        Self::from_object(value, data_contract)
    }
    #[cfg(feature = "value-conversion")]
    fn from_object(
        raw_transition: Value,
        _data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(raw_transition).map_err(ProtocolError::ValueError)
    }
    #[cfg(feature = "value-conversion")]
    fn from_value_map(
        map: BTreeMap<String, Value>,
        _data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let value: Value = map.into();
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "value-conversion")]
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }
    #[cfg(feature = "value-conversion")]
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let value = platform_value::to_value(self)?;
        value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "json-conversion")]
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "value-conversion")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map()?.into())
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    fn fixture() -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::new([0xa1; 32]),
            identity_contract_nonce: 7,
            document_type_name: "user".to_string(),
            data_contract_id: Identifier::new([0xb2; 32]),
        })
    }

    // Note: `DocumentBaseTransition` derives `Serialize/Deserialize` without a
    // `#[serde(tag = ...)]` attribute, so the enum uses serde's default
    // externally-tagged shape: `{"V0": { ... }}`.

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        // `identityContractNonce` is `u64` — JSON has only one number type so
        // the U64 distinction is erased on the wire (the value-path test below
        // pins the typed variant). Identifiers render as base58 in JSON HR.
        assert_eq!(
            json,
            json!({
                "$baseFormatVersion": "0",
                    "$id": "Bswb3UyeD1pUTaGiE6WvqwFpJZsQSEY1xhJePCDTHdvp",
                    "$identityContractNonce": 7,
                    "$type": "user",
                    "$dataContractId": "D2ZcUbtpG5sKq7XLeB4YnpNnTGSptKCxTddoNeydzJQq",
            })
        );
        let recovered =
            <DocumentBaseTransition as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = ValueConvertible::to_object(&original).expect("to_object");
        let id = Identifier::new([0xa1; 32]);
        let data_contract_id = Identifier::new([0xb2; 32]);
        // Externally-tagged enum: `{"V0": {<fields>}}`. `7u64` keeps the typed
        // U64 variant for `identity_contract_nonce`.
        assert_eq!(
            value,
            platform_value!({
                "$baseFormatVersion": "0",
                    "$id": id,
                    "$identityContractNonce": 7u64,
                    "$type": "user",
                    "$dataContractId": data_contract_id,
            })
        );
        let recovered =
            <DocumentBaseTransition as ValueConvertible>::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
