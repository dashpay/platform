use serde_json::{Value, Value as JsonValue};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Write,
};

use crate::{
    document::document_transition::DocumentTransition,
    prelude::DataContract,
    util::{
        json_schema::{Index, JsonSchemaExt},
        json_value::JsonValueExt,
    },
    ProtocolError,
};

#[macro_export]
/// Getter of Document Transition Base properties
macro_rules! get_from_transition {
    ($document_transition:expr, $property:ident) => {
        match $document_transition {
            DocumentTransition::Create(d) => &d.base.$property,
            DocumentTransition::Delete(d) => &d.base.$property,
            DocumentTransition::Replace(d) => &d.base.$property,
        }
    };
}

/// Finds duplicates of indices in Document Transitions.
pub fn find_duplicates_by_indices<'a>(
    document_raw_transitions: impl IntoIterator<Item = &'a JsonValue>,
    data_contract: &DataContract,
) -> Result<Vec<&'a JsonValue>, ProtocolError> {
    #[derive(Debug)]
    struct Group<'a> {
        transitions: Vec<&'a JsonValue>,
        indices: Vec<Index>,
    }
    let mut groups: HashMap<&'a str, Group> = HashMap::new();

    for dt in document_raw_transitions.into_iter() {
        let document_type = dt.get_string("$type")?;
        match groups.entry(document_type) {
            Entry::Occupied(mut o) => {
                o.get_mut().transitions.push(dt);
            }
            Entry::Vacant(v) => {
                v.insert(Group {
                    transitions: vec![dt],
                    indices: get_unique_indices(document_type, data_contract),
                });
            }
        };
    }

    let mut found_group_duplicates: Vec<&'a JsonValue> = vec![];
    for (_, group) in groups
        .iter()
        // Filter out groups without unique indices
        .filter(|(_, group)| !group.indices.is_empty())
        // Filter out group with only one object
        .filter(|(_, group)| group.transitions.len() > 1)
    {
        for transition in group.transitions.as_slice() {
            let transition_id = transition.get_bytes("$id")?;

            let mut found_duplicates: Vec<&'a JsonValue> = vec![];
            for transition_to_check in group
                .transitions
                .iter()
                // Exclude current transition from search
                .filter(|t| t.get_bytes("$id").unwrap() != transition_id)
            {
                if is_duplicate_by_indices(transition, transition_to_check, &group.indices) {
                    found_duplicates.push(transition_to_check)
                }
            }
            found_group_duplicates.extend(found_duplicates);
        }
    }

    Ok(found_group_duplicates)
}

fn is_duplicate_by_indices(
    original_transition: &JsonValue,
    transition_to_check: &JsonValue,
    type_indices: &[Index],
) -> bool {
    for index in type_indices {
        for property in index.properties.iter() {
            let original = original_transition
                .get(&property.name)
                .unwrap_or(&JsonValue::Null);
            let to_check = transition_to_check
                .get(&property.name)
                .unwrap_or(&JsonValue::Null);

            if original != to_check {
                return false;
            }
        }
    }
    true
}

fn get_unique_indices(document_type: &str, data_contract: &DataContract) -> Vec<Index> {
    let indices = data_contract
        .get_document_schema(document_type)
        .unwrap()
        .get_indices();
    indices
        // TODO should we panic or we should return and error or empty vector
        .expect("error while getting indices from json schema")
        .into_iter()
        .filter(|i| i.unique)
        .collect()
}

fn get_data_property(document_transition: &DocumentTransition, property_name: &str) -> String {
    match document_transition {
        DocumentTransition::Delete(_) => String::from(""),
        DocumentTransition::Create(dt_create) => match &dt_create.data {
            None => String::from(""),
            Some(data) => data
                .get(property_name)
                .unwrap_or(&Value::String(String::from("")))
                .to_string(),
        },
        DocumentTransition::Replace(dt_replace) => match &dt_replace.data {
            None => String::from(""),
            Some(data) => data
                .get(property_name)
                .unwrap_or(&Value::String(String::from("")))
                .to_string(),
        },
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::{prelude::*, util::string_encoding::Encoding};

    use super::find_duplicates_by_indices;

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

        let mut data_contract = DataContract::default();
        data_contract.set_document_schema("indexedDocument".to_string(), document_def.clone());
        data_contract.set_document_schema("singleDocument".to_string(), document_def);

        let id_1 = Identifier::from_string(
            "AoqSTh5Bg6Fo26NaCRVoPP1FiDQ1ycihLkjQ75MYJziV",
            Encoding::Base58,
        )
        .unwrap();
        let document_raw_transition_1 = json!(
            {
                "$id": id_1.as_bytes(),
                "$type": "indexedDocument",
                "$action": 0,
                "$dataContractId": "F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg",
                "name": "Leon",
                "lastName": "Birkin",
                "$entropy": "hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=",
              }
        );

        let id_2 = Identifier::from_string(
            "3GDfArJJdHMviaRd5ta4F2EB7LN9RgbMKLAfjAxZEaUG",
            Encoding::Base58,
        )
        .unwrap();
        let document_create_transition_2 = json!(
            {
                "$id": id_2.as_bytes(),
                "$type": "indexedDocument",
                "$action": 0,
                "$dataContractId": "F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg",
                "name": "William",
                "lastName": "Birkin",
                "$entropy": "hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=",
              }
        );

        let duplicates = find_duplicates_by_indices(
            [&document_raw_transition_1, &document_create_transition_2],
            &data_contract,
        )
        .expect("the error shouldn't be returned");
        assert!(duplicates.len() == 2);
    }
}
