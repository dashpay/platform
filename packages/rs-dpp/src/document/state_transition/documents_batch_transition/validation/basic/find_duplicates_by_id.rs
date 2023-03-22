use anyhow::Context;

use platform_value::Value;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
struct IdFingerprint<'a> {
    document_type: &'a str,
    id: [u8; 32],
}

/// Find the duplicates in the collection of Document Transitions
pub fn find_duplicates_by_id<'a>(
    document_transitions: impl IntoIterator<Item = &'a Value>,
) -> Result<Vec<&'a Value>, anyhow::Error> {
    let mut fingerprints: BTreeMap<IdFingerprint, &'a Value> = BTreeMap::new();
    let mut duplicates: Vec<&'a Value> = vec![];

    for transition in document_transitions {
        let fingerprint = create_fingerprint(transition)
            .context("can't create fingerprint from a document transition")?;

        match fingerprints.entry(fingerprint) {
            Entry::Occupied(val) => {
                duplicates.push(val.get());
                duplicates.push(transition);
            }
            Entry::Vacant(v) => {
                v.insert(transition);
            }
        }
    }
    Ok(duplicates)
}

fn create_fingerprint(document_transition: &Value) -> Result<IdFingerprint, anyhow::Error> {
    let map = document_transition.to_map().context("should be a map")?;
    Ok(IdFingerprint {
        document_type: Value::inner_text_value(map, "$type")?,
        id: Value::inner_hash256_value(map, "$id")?,
    })
}

#[cfg(test)]
mod test {
    use crate::document::document_transition::DocumentTransitionObjectLike;
    use crate::{
        document::document_transition::{
            DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
        },
        tests::utils::generate_random_identifier_struct,
    };

    use super::find_duplicates_by_id;

    #[test]
    fn test_duplicates() {
        let mut dt_create = DocumentCreateTransition::default();
        dt_create.base.id = generate_random_identifier_struct();
        dt_create.base.document_type_name = String::from("a");

        let dt_create_duplicate = dt_create.clone();

        let mut dt_replace = DocumentReplaceTransition::default();
        dt_replace.base.id = generate_random_identifier_struct();
        dt_replace.base.document_type_name = String::from("b");

        let mut dt_delete = DocumentDeleteTransition::default();
        dt_delete.base.id = generate_random_identifier_struct();
        dt_delete.base.document_type_name = String::from("c");

        let create_json = dt_create.to_object().unwrap();
        let dt_create_duplicate_json = dt_create_duplicate.to_object().unwrap();
        let dt_replace_json = dt_replace.to_object().unwrap();
        let dt_delete_json = dt_delete.to_object().unwrap();

        let input = vec![
            create_json,
            dt_create_duplicate_json,
            dt_replace_json,
            dt_delete_json,
        ];

        let duplicates = find_duplicates_by_id(input.iter()).unwrap();
        assert_eq!(duplicates.len(), 2);
    }
}
