use anyhow::anyhow;
use serde_json::Value as JsonValue;
use sha2::digest::generic_array::functional::FunctionalSequence;
use std::collections::{hash_map::Entry, HashMap};

use crate::document::document_transition::{
    DocumentBaseTransition, DocumentTransition, DocumentTransitionObjectLike,
};
use crate::util::string_encoding::Encoding;

/// Find the duplicates in the collection of Document Transitions
pub fn find_duplicates_by_id<'a>(
    document_transitions: impl IntoIterator<Item = &'a JsonValue>,
) -> Result<Vec<JsonValue>, anyhow::Error> {
    let mut fingerprints: HashMap<String, JsonValue> = HashMap::new();
    let mut duplicates: Vec<JsonValue> = vec![];

    for transition in document_transitions {
        let fingerprint = create_fingerprint(&transition).ok_or(anyhow!(
            "Can't create fingerprint from a document transition"
        ))?;
        match fingerprints.entry(fingerprint.clone()) {
            Entry::Occupied(val) => {
                duplicates.push(val.get().clone());
            }
            Entry::Vacant(v) => {
                v.insert(transition.clone());
            }
        }
    }
    Ok(duplicates)
}

fn create_fingerprint(document_transition: &JsonValue) -> Option<String> {
    Some(format!(
        "{}:{}",
        document_transition.as_object()?.get("$type")?,
        document_transition.as_object()?.get("id")?,
    ))
}

#[cfg(test)]
mod test {
    use crate::{
        document::document_transition::{
            DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
            DocumentTransition,
        },
        tests::utils::generate_random_identifier_struct,
    };
    use crate::document::document_transition::{DocumentTransitionObjectLike, JsonValue};

    use super::find_duplicates_by_id;

    #[test]
    fn test_duplicates() {
        let mut dt_create = DocumentCreateTransition::default();
        dt_create.base.id = generate_random_identifier_struct();
        dt_create.base.document_type = String::from("a");

        let dt_create_duplicate = dt_create.clone();

        let mut dt_replace = DocumentReplaceTransition::default();
        dt_replace.base.id = generate_random_identifier_struct();
        dt_replace.base.document_type = String::from("b");

        let mut dt_delete = DocumentDeleteTransition::default();
        dt_delete.base.id = generate_random_identifier_struct();
        dt_delete.base.document_type = String::from("c");

        let create_json = dt_create.to_json().unwrap();
        let dt_create_duplicate_json = dt_create_duplicate.to_json().unwrap();
        let dt_replace_json = dt_replace.to_json().unwrap();
        let dt_delete_json = dt_delete.to_json().unwrap();

        let input = vec![
            create_json,
            dt_create_duplicate_json,
            dt_replace_json,
            dt_delete_json,
        ];

        let duplicates = find_duplicates_by_id(input.iter()).unwrap();
        assert_eq!(duplicates.len(), 1);
    }
}
