use serde_json::Value;
use std::collections::{hash_map::Entry, HashMap};

use crate::{
    document::document_transition::DocumentTransition,
    prelude::DataContract,
    util::json_schema::{Index, JsonSchemaExt},
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
    document_transitions: &'a [DocumentTransition],
    data_contract: &'a DataContract,
) -> Vec<&'a DocumentTransition> {
    #[derive(Debug)]
    struct Group<'a> {
        transitions: Vec<&'a DocumentTransition>,
        indices: Vec<Index>,
    }
    let mut groups: HashMap<&'a str, Group> = HashMap::new();

    for dt in document_transitions {
        let dt_type = get_from_transition!(dt, document_type);
        match groups.entry(dt_type) {
            Entry::Occupied(mut o) => {
                o.get_mut().transitions.push(dt);
            }
            Entry::Vacant(v) => {
                v.insert(Group {
                    transitions: vec![dt],
                    indices: get_unique_indices(dt_type, data_contract),
                });
            }
        };
    }

    let mut found_group_duplicates: Vec<&'a DocumentTransition> = vec![];
    for (_, group) in groups
        .iter()
        // Filter out groups without unique indices
        .filter(|(_, group)| !group.indices.is_empty())
        // Filter out group with only one object
        .filter(|(_, group)| group.transitions.len() > 1)
    {
        for transition in group.transitions.as_slice() {
            let mut found_duplicates: Vec<&'a DocumentTransition> = vec![];
            for transition_to_check in group
                .transitions
                .iter()
                // Exclude current transition from search
                .filter(|t| get_from_transition!(t, id) != get_from_transition!(transition, id))
            {
                if is_duplicate_by_indices(transition, transition_to_check, &group.indices) {
                    found_duplicates.push(transition_to_check)
                }
            }
            found_group_duplicates.extend(found_duplicates);
        }
    }

    found_group_duplicates
}

fn is_duplicate_by_indices(
    original_transition: &DocumentTransition,
    transition_to_check: &DocumentTransition,
    type_indices: &[Index],
) -> bool {
    for index in type_indices {
        let mut original_hash = String::new();
        let mut hash_to_check = String::new();

        for (property_name, _) in index.properties.iter().flatten() {
            original_hash.push_str(&format!(
                ":{}",
                get_data_property(original_transition, property_name)
            ));
            hash_to_check.push_str(&format!(
                ":{}",
                get_data_property(transition_to_check, property_name)
            ));
        }
        if original_hash == hash_to_check {
            return true;
        }
    }
    false
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

    use crate::{
        document::document_transition::{
            DocumentCreateTransition, DocumentTransition, DocumentTransitionObjectLike,
        },
        prelude::*,
    };

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

        let doc_create_transition = DocumentCreateTransition::from_json_str(
            &json!(
                {
                    "$id": "AoqSTh5Bg6Fo26NaCRVoPP1FiDQ1ycihLkjQ75MYJziV",
                    "$type": "indexedDocument",
                    "$action": 0,
                    "$dataContractId": "F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg",
                    "name": "Leon",
                    "lastName": "Birkin",
                    "$entropy": "hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=",
                  }
            )
            .to_string(),
            data_contract.clone(),
        )
        .unwrap();
        let document_create_transition_2 = DocumentCreateTransition::from_json_str(
            &json!(
                {
                    "$id": "3GDfArJJdHMviaRd5ta4F2EB7LN9RgbMKLAfjAxZEaUG",
                    "$type": "indexedDocument",
                    "$action": 0,
                    "$dataContractId": "F719NPkos8a2VqxSPv4co4F8owh9qBbYEMJ1gzyLANtg",
                    "name": "William",
                    "lastName": "Birkin",
                    "$entropy": "hxlmtQ34oR/lkql7AUQ13P5kS8OaX2BheksnPBIpxLc=",
                  }
            )
            .to_string(),
            data_contract.clone(),
        )
        .unwrap();

        let transitions: Vec<DocumentTransition> = vec![
            DocumentTransition::Create(doc_create_transition),
            DocumentTransition::Create(document_create_transition_2),
        ];

        let duplicates = find_duplicates_by_indices(&transitions, &data_contract);
        assert!(duplicates.len() == 2);
    }
}
