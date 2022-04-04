use std::collections::{hash_map::Entry, HashMap};

use serde_json::Value;

use crate::{
    document::document_transition::DocumentTransition,
    prelude::DataContract,
    util::json_schema::{get_indices_from_json_schema, Index},
};

macro_rules! get {
    ($document_transition:ident, $property:ident) => {
        match $document_transition {
            DocumentTransition::Create(d) => &d.base.$property,
            DocumentTransition::Delete(d) => &d.base.$property,
            DocumentTransition::Replace(d) => &d.base.$property,
        }
    };
}

pub fn find_duplicates_by_indices<'a>(
    document_transitions: &'a [DocumentTransition],
    data_contract: &'a DataContract,
) -> Vec<&'a DocumentTransition> {
    struct Group<'a> {
        transitions: Vec<&'a DocumentTransition>,
        indices: Vec<Index>,
    }
    let mut groups: HashMap<&'a str, Group> = HashMap::new();

    for dt in document_transitions {
        let dt_type = get!(dt, document_type);

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
        .filter(|(_, group)| !group.indices.is_empty())
        .filter(|(_, group)| group.transitions.len() > 1)
    {
        for transition in group.transitions.as_slice() {
            let mut found_duplicates: Vec<&'a DocumentTransition> = vec![];
            for transition_to_check in group
                .transitions
                .iter()
                .filter(|t| get!(t, id) != get!(transition, id))
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

        for (property_name, _) in index.properties.iter() {
            original_hash.push_str(&format!(
                ":{}",
                get_data_property(original_transition, property_name)
            ));
            hash_to_check.push_str(&format!(
                ":{}",
                get_data_property(transition_to_check, property_name)
            ));
        }
        if original_hash != hash_to_check {
            return false;
        }
    }
    true
}

fn get_unique_indices(document_type: &str, data_contract: &DataContract) -> Vec<Index> {
    let indices =
        get_indices_from_json_schema(data_contract.get_document_schema(document_type).unwrap());
    indices
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
