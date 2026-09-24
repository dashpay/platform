mod v0;

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
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "Index::extract_contested_values".to_string(),
                known_versions: vec![0],
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
        ByteArrayPropertySizes, DocumentPropertyType,
    };

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
        let property = |property_type| DocumentProperty {
            property_type,
            required: true,
            transient: false,
            required_since: None,
            distinct_from: None,
            encrypted_for: None,
        };
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
