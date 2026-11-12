mod v0;
mod v1;

use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::Index;
use crate::ProtocolError;
use indexmap::IndexMap;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Index {
    /// The values a contest on this index names its resource by: those of
    /// [`Self::extract_values`], with every identifier property written as
    /// `Value::Identifier` from protocol version 14. Validation also accepts an identifier as
    /// bytes or as an array of 32 byte values, and the index keys store all of them alike,
    /// but a contest's poll is hashed from these values: two contenders writing the same
    /// identifier in two forms would otherwise name one contest with two polls, each with
    /// its own prefunded balance and end date. `document_properties` are the document type's
    /// flattened properties. Before 14 the values are taken as given.
    ///
    /// From protocol version 15 the value of every property is written one way, the way its
    /// type keys it: an integer as its type's own integer, an integer or a float of a float
    /// property as a float, bytes in any form as bytes, `"\0"` as the empty string it shares
    /// a key with. A contest is stored at the path of its index keys, so values with one key
    /// are one contest, and they now name one poll.
    ///
    /// # Parameters
    /// * `data`: the document's properties.
    /// * `document_properties`: the document type's flattened properties.
    /// * `platform_version`: the platform version.
    ///
    /// # Returns
    /// The index values of the contest, one per index property, in index order.
    pub fn extract_contested_values(
        &self,
        data: &BTreeMap<String, Value>,
        document_properties: &IndexMap<String, DocumentProperty>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Value>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .canonical_contested_index_values
        {
            None => Ok(self.extract_values(data)),
            Some(0) => Ok(self.extract_contested_values_v0(data, document_properties)),
            Some(1) => Ok(self.extract_contested_values_v1(data, document_properties)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "Index::extract_contested_values".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

/// The identifier a contest's index value names, in every form validation accepts for an
/// identifier property: `Value::Identifier`, 32 bytes, or an array of 32 byte values. `None`
/// for any other value, a base58 string included.
pub fn contested_index_identifier(value: &Value) -> Option<[u8; 32]> {
    match value {
        Value::Identifier(bytes) | Value::Bytes32(bytes) => Some(*bytes),
        Value::Bytes(bytes) => <[u8; 32]>::try_from(bytes.as_slice()).ok(),
        Value::Array(items) if items.len() == 32 => items
            .iter()
            .map(|item| item.to_integer::<u8>().ok())
            .collect::<Option<Vec<u8>>>()
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::index::tests::make_index;
    use crate::data_contract::document_type::property::{
        ByteArrayPropertySizes, DocumentPropertyType, StringPropertySizes,
    };

    fn property(property_type: DocumentPropertyType) -> DocumentProperty {
        DocumentProperty {
            property_type,
            required: true,
            transient: false,
            required_since: None,
            distinct_from: None,
            encrypted_for: None,
            generated_from: None,
            revealed_reference: None,
        }
    }

    /// The values a contest over one property of `property_type` names itself by, for a
    /// document holding `value`.
    fn contest_values(
        property_type: &DocumentPropertyType,
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Vec<Value> {
        make_index("contested", vec![("value", true)], true)
            .extract_contested_values(
                &BTreeMap::from([("value".to_string(), value.clone())]),
                &IndexMap::from([("value".to_string(), property(property_type.clone()))]),
                platform_version,
            )
            .expect("values")
    }

    /// From protocol version 15 the forms of a value that share a key in the paths of a vote
    /// poll are written one way, so they name one poll: the way a query of the contest decodes
    /// that key, so a voter reading the contest names the same poll. Protocol version 14 takes
    /// them as given, identifiers apart.
    #[test]
    fn should_write_contest_values_that_share_a_key_one_way_from_version_15() {
        let integer_forms = || vec![Value::U8(5), Value::I16(5), Value::U64(5), Value::I128(5)];
        let bytes = ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        };
        let string = StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        };
        let float_forms = {
            let mut forms = integer_forms();
            forms.push(Value::Float(5.0));
            forms
        };
        let cases: Vec<(DocumentPropertyType, Vec<Value>, Value)> = vec![
            (DocumentPropertyType::U128, integer_forms(), Value::U128(5)),
            (DocumentPropertyType::I128, integer_forms(), Value::I128(5)),
            (DocumentPropertyType::U64, integer_forms(), Value::U64(5)),
            (DocumentPropertyType::I64, integer_forms(), Value::I64(5)),
            (DocumentPropertyType::U32, integer_forms(), Value::U32(5)),
            (DocumentPropertyType::I32, integer_forms(), Value::I32(5)),
            (DocumentPropertyType::U16, integer_forms(), Value::U16(5)),
            (DocumentPropertyType::I16, integer_forms(), Value::I16(5)),
            (DocumentPropertyType::U8, integer_forms(), Value::U8(5)),
            (DocumentPropertyType::I8, integer_forms(), Value::I8(5)),
            (DocumentPropertyType::Date, integer_forms(), Value::U64(5)),
            (DocumentPropertyType::F64, float_forms, Value::Float(5.0)),
            (
                DocumentPropertyType::String(string.clone()),
                vec![Value::Text(String::new()), Value::Text("\0".to_string())],
                Value::Text(String::new()),
            ),
            (
                DocumentPropertyType::String(string),
                vec![Value::Text("dash".to_string())],
                Value::Text("dash".to_string()),
            ),
            (
                DocumentPropertyType::ByteArray(bytes.clone()),
                vec![
                    Value::Bytes(vec![1, 2, 3]),
                    Value::Array(vec![Value::U8(1), Value::U8(2), Value::U8(3)]),
                    Value::Text("AQID".to_string()),
                ],
                Value::Bytes(vec![1, 2, 3]),
            ),
            (
                DocumentPropertyType::ByteArray(bytes.clone()),
                vec![
                    Value::Bytes(vec![7; 32]),
                    Value::Bytes32([7; 32]),
                    Value::Identifier([7; 32]),
                ],
                Value::Bytes(vec![7; 32]),
            ),
            // No bytes have the empty key, as a missing value has
            (
                DocumentPropertyType::ByteArray(bytes),
                vec![Value::Bytes(vec![]), Value::Array(vec![])],
                Value::Null,
            ),
            (
                DocumentPropertyType::Identifier,
                vec![
                    Value::Identifier([0x7A; 32]),
                    Value::Bytes32([0x7A; 32]),
                    Value::Bytes(vec![0x7A; 32]),
                    Value::Array(vec![Value::U8(0x7A); 32]),
                    Value::Text(bs58::encode([0x7A; 32]).into_string()),
                ],
                Value::Identifier([0x7A; 32]),
            ),
            (
                DocumentPropertyType::Boolean,
                vec![Value::Bool(true)],
                Value::Bool(true),
            ),
        ];
        let latest = PlatformVersion::latest();
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");

        for (property_type, forms, expected) in cases {
            for form in forms {
                assert_eq!(
                    contest_values(&property_type, &form, latest),
                    vec![expected.clone()],
                    "{property_type:?} {form:?}"
                );
                let key = property_type
                    .encode_value_for_tree_keys(&form)
                    .expect("expected the key of the value");
                assert_eq!(
                    property_type
                        .decode_value_for_tree_keys(&key)
                        .expect("expected the value of the key"),
                    expected,
                    "{property_type:?} {form:?}"
                );
                if !matches!(property_type, DocumentPropertyType::Identifier) {
                    assert_eq!(
                        contest_values(&property_type, &form, version_14),
                        vec![form],
                        "{property_type:?}"
                    );
                }
            }
        }
    }

    /// A value its property's type does not key is in no contest, and is left as it is.
    #[test]
    fn should_leave_a_contest_value_its_type_does_not_key_as_it_is() {
        let latest = PlatformVersion::latest();
        for (property_type, value) in [
            (DocumentPropertyType::U8, Value::I64(-1)),
            (DocumentPropertyType::U8, Value::U64(256)),
            (DocumentPropertyType::I64, Value::Text("5".to_string())),
            (DocumentPropertyType::F64, Value::Text("5".to_string())),
            (DocumentPropertyType::Boolean, Value::U8(1)),
            (DocumentPropertyType::Identifier, Value::Bytes(vec![1; 31])),
            (DocumentPropertyType::I64, Value::Null),
        ] {
            assert_eq!(
                contest_values(&property_type, &value, latest),
                vec![value.clone()],
                "{property_type:?} {value:?}"
            );
        }

        // A property path or a system property is no top-level property of the type
        let index = make_index("contested", vec![("$ownerId", true)], true);
        let data = BTreeMap::from([("$ownerId".to_string(), Value::Bytes(vec![1; 32]))]);
        assert_eq!(
            index
                .extract_contested_values(&data, &IndexMap::new(), latest)
                .expect("values"),
            vec![Value::Bytes(vec![1; 32])]
        );
    }

    /// From protocol version 14 an identifier index value in any accepted form is written as
    /// `Value::Identifier`, so every contender of one contest names it with the same poll; a
    /// byte array property keeps its bytes. Before 14 the values are taken as given.
    #[test]
    fn should_write_identifier_contest_values_as_identifiers_from_version_14() {
        let index = make_index(
            "byTargetContract",
            vec![("targetContractId", true), ("salt", true)],
            true,
        );
        let document_properties = IndexMap::from([
            (
                "targetContractId".to_string(),
                property(DocumentPropertyType::Identifier),
            ),
            (
                "salt".to_string(),
                property(DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                    min_size: Some(32),
                    max_size: Some(32),
                })),
            ),
        ]);
        let latest = PlatformVersion::latest();
        let before_14 = PlatformVersion::get(13).expect("protocol version 13");

        for target in [
            Value::Identifier([0x7A; 32]),
            Value::Bytes32([0x7A; 32]),
            Value::Bytes(vec![0x7A; 32]),
            Value::Array(vec![Value::U8(0x7A); 32]),
        ] {
            let data = BTreeMap::from([
                ("targetContractId".to_string(), target.clone()),
                ("salt".to_string(), Value::Bytes(vec![0x01; 32])),
            ]);
            assert_eq!(
                index
                    .extract_contested_values(&data, &document_properties, latest)
                    .expect("values"),
                vec![Value::Identifier([0x7A; 32]), Value::Bytes(vec![0x01; 32])],
                "{target:?}"
            );
            assert_eq!(
                index
                    .extract_contested_values(&data, &document_properties, before_14)
                    .expect("values"),
                vec![target, Value::Bytes(vec![0x01; 32])]
            );
        }
    }
}
