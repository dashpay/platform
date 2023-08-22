use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::{Value, ValueMap};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use crate::{
    prelude::DataContract,
    ProtocolError,
};
use crate::data_contract::base::DataContractBaseMethodsV0;
use crate::data_contract::document_type::Index;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;

#[macro_export]
/// Getter of Document Transition Base properties
macro_rules! get_from_transition {
    ($document_transition:expr, $property:ident) => {
        match $document_transition {
            DocumentTransition::Create(d) => &d.base().$property,
            DocumentTransition::Delete(d) => &d.base().$property,
            DocumentTransition::Replace(d) => &d.base().$property,
        }
    };
}

#[macro_export]
/// Getter of Document Transition Base properties
macro_rules! get_from_transition_action {
    ($document_transition_action:expr, $property:ident) => {
        match $document_transition_action {
            DocumentTransitionAction::CreateAction(d) => &d.base().$property,
            DocumentTransitionAction::DeleteAction(d) => &d.base().$property,
            DocumentTransitionAction::ReplaceAction(d) => &d.base().$property,
        }
    };
}

/// Finds duplicates of indices in Document Transitions.
pub fn find_duplicates_by_indices<'a>(
    raw_extended_documents: impl IntoIterator<Item = &'a Value>,
    data_contract: &'a DataContract,
) -> Result<Vec<&'a Value>, ProtocolError> {
    #[derive(Debug)]
    struct Group<'a> {
        transitions: Vec<&'a Value>,
        indices: &'a [Index],
    }
    let mut groups: BTreeMap<&'a str, Group> = BTreeMap::new();

    for dt in raw_extended_documents.into_iter() {
        let document_type_name = dt.get_str("$type")?;
        let document_type = data_contract.document_type_for_name(document_type_name)?;
        match groups.entry(document_type_name) {
            Entry::Occupied(mut o) => {
                o.get_mut().transitions.push(dt);
            }
            Entry::Vacant(v) => {
                v.insert(Group {
                    transitions: vec![dt],
                    indices: document_type.indices.as_slice(),
                });
            }
        };
    }

    let mut found_group_duplicates: Vec<&'a Value> = vec![];
    for (_, group) in groups
        .iter()
        // Filter out groups without unique indices
        .filter(|(_, group)| !group.indices.is_empty())
        // Filter out group with only one object
        .filter(|(_, group)| group.transitions.len() > 1)
    {
        for (i, value1) in group.transitions.iter().enumerate() {
            let object1 = value1.to_map().map_err(ProtocolError::ValueError)?;
            let mut found_duplicates: Vec<&'a Value> = vec![];
            for value2 in group
                .transitions
                .split_at(i + 1)
                .1 // we get the second part
                .iter()
            {
                let object2 = value2.to_map().map_err(ProtocolError::ValueError)?;
                if is_duplicate_by_indices(object1, object2, group.indices) {
                    found_duplicates.push(value1);
                    found_duplicates.push(value2);
                }
            }
            found_group_duplicates.extend(found_duplicates);
        }
    }

    Ok(found_group_duplicates)
}

fn get_data_property(document_transition: &DocumentTransition, property_name: &str) -> String {
    match document_transition {
        DocumentTransition::Delete(_) => String::from(""),
        DocumentTransition::Create(dt_create) => match &dt_create.data {
            None => String::from(""),
            Some(data) => data
                .get_optional_string(property_name)
                .ok()
                .flatten()
                .unwrap_or(String::from("")),
        },
        DocumentTransition::Replace(dt_replace) => match &dt_replace.data {
            None => String::from(""),
            Some(data) => data
                .get_optional_string(property_name)
                .ok()
                .flatten()
                .unwrap_or(String::from("")),
        },
    }
}

fn is_duplicate_by_indices(object1: &ValueMap, object2: &ValueMap, type_indices: &[Index]) -> bool {
    type_indices
        .iter()
        .any(|index| index.objects_are_conflicting(object1, object2))
}

#[cfg(test)]
mod test {
    use platform_value::string_encoding::Encoding;
    use platform_value::Value;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::convert::TryInto;

    use crate::data_contract::document_type::DocumentTypeRef;
    use crate::prelude::*;

    use super::find_duplicates_by_indices;

    fn setup_test() {
        let document_def = json!(                {
                            "indices": [
                              {
                                "name": "ownerIdLastName",
                                "properties": [
                                  {"$ownerId": "asc"},
                                  {"lastName": "asc"},
                                ],
                                "unique": true,
                              },
                            ],
                            "properties": {
                              "firstName": {
                                "type": "string",
                              },
                              "lastName": {
                                "type": "string",
                              },
                            },
                            "required": ["lastName"],
                            "additionalProperties": false,
                          }
        );

        let mut data_contract = DataContract::default();
        data_contract.set_document_schema("singleDocument".to_string(), document_def);
    }

    #[test]
    fn test_non_required_field_not_being_present_doesnt_find_index_duplicate() {
        let document_def = json!(                {
                            "indices": [
                              {
                                "name": "ownerIdLastName",
                                "properties": [
                                  {"$ownerId": "asc"},
                                  {"lastName": "asc"},
                                ],
                                "unique": true,
                              },
                            ],
                            "properties": {
                              "firstName": {
                                "type": "string",
                              },
                              "lastName": {
                                "type": "string",
                              },
                            },
                            "required": ["lastName"],
                            "additionalProperties": false,
                          }
        );

        let document_def_value: Value = document_def.clone().into();

        let document_type = DocumentTypeRef::from_platform_value(
            Default::default(),
            "indexedDocument",
            document_def_value.to_map().expect("expected a map"),
            &BTreeMap::new(),
            false,
            false,
        )
        .expect("expected a document type");

        let mut data_contract = DataContract::default();
        data_contract
            .document_types
            .insert("indexedDocument".to_string(), document_type);
        data_contract.set_document_schema("indexedDocument".to_string(), document_def.clone());
        data_contract.set_document_schema("singleDocument".to_string(), document_def);

        let id_1 = Identifier::from_string(
            "AoqSTh5Bg6Fo26NaCRVoPP1FiDQ1ycihLkjQ75MYJziV",
            Encoding::Base58,
        )
        .unwrap();
        let document_raw_transition_1: Value = BTreeMap::from([
            ("$id".to_string(), Value::Identifier(id_1.to_buffer())),
            (
                "$type".to_string(),
                Value::Text("indexedDocument".to_string()),
            ),
            ("$action".to_string(), Value::U8(0)),
            (
                "$dataContractId".to_string(),
                Value::Identifier(
                    bs58::decode("F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg")
                        .into_vec()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
            ("name".to_string(), Value::Text("Leon".to_string())),
            ("lastName".to_string(), Value::Text("Birkin".to_string())),
            (
                "$entropy".to_string(),
                Value::Bytes32(
                    base64::decode("hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=")
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
        ])
        .into();

        let id_2 = Identifier::from_string(
            "3GDfArJJdHMviaRd5ta4F2EB7LN9RgbMKLAfjAxZEaUG",
            Encoding::Base58,
        )
        .unwrap();

        let document_create_transition_2: Value = BTreeMap::from([
            ("$id".to_string(), Value::Identifier(id_2.to_buffer())),
            (
                "$type".to_string(),
                Value::Text("indexedDocument".to_string()),
            ),
            ("$action".to_string(), Value::U8(0)),
            (
                "$dataContractId".to_string(),
                Value::Identifier(
                    bs58::decode("F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg")
                        .into_vec()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
            ("name".to_string(), Value::Text("William".to_string())),
            ("lastName".to_string(), Value::Text("Birkin".to_string())),
            (
                "$entropy".to_string(),
                Value::Bytes32(
                    base64::decode("hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=")
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
        ])
        .into();

        let duplicates = find_duplicates_by_indices(
            [&document_raw_transition_1, &document_create_transition_2],
            &data_contract,
        )
        .expect("the error shouldn't be returned");
        assert_eq!(duplicates.len(), 0);
    }

    #[test]
    fn test_find_duplicates_by_indices() {
        let document_def = json!(                {
                            "indices": [
                              {
                                "name": "ownerIdLastName",
                                "properties": [
                                  {"$ownerId": "asc"},
                                  {"lastName": "asc"},
                                ],
                                "unique": true,
                              },
                            ],
                            "properties": {
                              "firstName": {
                                "type": "string",
                              },
                              "lastName": {
                                "type": "string",
                              },
                            },
                            "required": ["lastName"],
                            "additionalProperties": false,
                          }
        );

        let document_def_value: Value = document_def.clone().into();

        let document_type = DocumentTypeRef::from_platform_value(
            Default::default(),
            "indexedDocument",
            document_def_value.to_map().expect("expected a map"),
            &BTreeMap::new(),
            false,
            false,
        )
        .expect("expected a document type");

        let mut data_contract = DataContract::default();
        data_contract
            .document_types
            .insert("indexedDocument".to_string(), document_type);
        data_contract.set_document_schema("indexedDocument".to_string(), document_def.clone());
        data_contract.set_document_schema("singleDocument".to_string(), document_def);

        let id_1 = Identifier::from_string(
            "AoqSTh5Bg6Fo26NaCRVoPP1FiDQ1ycihLkjQ75MYJziV",
            Encoding::Base58,
        )
        .unwrap();
        let document_raw_transition_1: Value = BTreeMap::from([
            ("$ownerId".to_string(), Value::Identifier(id_1.to_buffer())),
            ("$id".to_string(), Value::Identifier(id_1.to_buffer())),
            (
                "$type".to_string(),
                Value::Text("indexedDocument".to_string()),
            ),
            ("$action".to_string(), Value::U8(0)),
            (
                "$dataContractId".to_string(),
                Value::Identifier(
                    bs58::decode("F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg")
                        .into_vec()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
            ("name".to_string(), Value::Text("Leon".to_string())),
            ("lastName".to_string(), Value::Text("Birkin".to_string())),
            (
                "$entropy".to_string(),
                Value::Bytes32(
                    base64::decode("hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=")
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
        ])
        .into();

        let id_2 = Identifier::from_string(
            "3GDfArJJdHMviaRd5ta4F2EB7LN9RgbMKLAfjAxZEaUG",
            Encoding::Base58,
        )
        .unwrap();

        let document_create_transition_2: Value = BTreeMap::from([
            ("$ownerId".to_string(), Value::Identifier(id_1.to_buffer())),
            ("$id".to_string(), Value::Identifier(id_2.to_buffer())),
            (
                "$type".to_string(),
                Value::Text("indexedDocument".to_string()),
            ),
            ("$action".to_string(), Value::U8(0)),
            (
                "$dataContractId".to_string(),
                Value::Identifier(
                    bs58::decode("F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg")
                        .into_vec()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
            ("name".to_string(), Value::Text("William".to_string())),
            ("lastName".to_string(), Value::Text("Birkin".to_string())),
            (
                "$entropy".to_string(),
                Value::Bytes32(
                    base64::decode("hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=")
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
        ])
        .into();

        let duplicates = find_duplicates_by_indices(
            [&document_raw_transition_1, &document_create_transition_2],
            &data_contract,
        )
        .expect("the error shouldn't be returned");
        assert_eq!(duplicates.len(), 2);
    }

    #[test]
    fn test_find_duplicates_by_single_value_indices() {
        let document_def = json!(                {
                            "indices": [
                              {
                                "name": "lastName",
                                "properties": [
                                  {"lastName": "asc"},
                                ],
                                "unique": true,
                              },
                            ],
                            "properties": {
                              "firstName": {
                                "type": "string",
                              },
                              "lastName": {
                                "type": "string",
                              },
                            },
                            "required": ["lastName"],
                            "additionalProperties": false,
                          }
        );

        let document_def_value: Value = document_def.clone().into();

        let document_type = DocumentTypeRef::from_platform_value(
            Default::default(),
            "indexedDocument",
            document_def_value.to_map().expect("expected a map"),
            &BTreeMap::new(),
            false,
            false,
        )
        .expect("expected a document type");

        let mut data_contract = DataContract::default();
        data_contract
            .document_types
            .insert("indexedDocument".to_string(), document_type);
        data_contract.set_document_schema("indexedDocument".to_string(), document_def.clone());
        data_contract.set_document_schema("singleDocument".to_string(), document_def);

        let id_1 = Identifier::from_string(
            "AoqSTh5Bg6Fo26NaCRVoPP1FiDQ1ycihLkjQ75MYJziV",
            Encoding::Base58,
        )
        .unwrap();
        let document_raw_transition_1: Value = BTreeMap::from([
            ("$id".to_string(), Value::Identifier(id_1.to_buffer())),
            (
                "$type".to_string(),
                Value::Text("indexedDocument".to_string()),
            ),
            ("$action".to_string(), Value::U8(0)),
            (
                "$dataContractId".to_string(),
                Value::Identifier(
                    bs58::decode("F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg")
                        .into_vec()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
            ("name".to_string(), Value::Text("Leon".to_string())),
            ("lastName".to_string(), Value::Text("Birkin".to_string())),
            (
                "$entropy".to_string(),
                Value::Bytes32(
                    base64::decode("hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=")
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
        ])
        .into();

        let id_2 = Identifier::from_string(
            "3GDfArJJdHMviaRd5ta4F2EB7LN9RgbMKLAfjAxZEaUG",
            Encoding::Base58,
        )
        .unwrap();

        let document_create_transition_2: Value = BTreeMap::from([
            ("$id".to_string(), Value::Identifier(id_2.to_buffer())),
            (
                "$type".to_string(),
                Value::Text("indexedDocument".to_string()),
            ),
            ("$action".to_string(), Value::U8(0)),
            (
                "$dataContractId".to_string(),
                Value::Identifier(
                    bs58::decode("F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg")
                        .into_vec()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
            ("name".to_string(), Value::Text("William".to_string())),
            ("lastName".to_string(), Value::Text("Birkin".to_string())),
            (
                "$entropy".to_string(),
                Value::Bytes32(
                    base64::decode("hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=")
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
            ),
        ])
        .into();

        let duplicates = find_duplicates_by_indices(
            [&document_raw_transition_1, &document_create_transition_2],
            &data_contract,
        )
        .expect("the error shouldn't be returned");
        assert_eq!(duplicates.len(), 2);
    }
}
