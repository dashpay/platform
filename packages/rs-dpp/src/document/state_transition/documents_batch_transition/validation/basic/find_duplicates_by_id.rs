use std::collections::{hash_map::Entry, HashMap};

use crate::document::document_transition::{DocumentBaseTransition, DocumentTransition};
use crate::util::string_encoding::Encoding;

/// Find the duplicates in the collection of Document Transitions
pub fn find_duplicates_by_id<'a>(
    document_transitions: impl IntoIterator<Item = &'a DocumentTransition>,
) -> Vec<&'a DocumentTransition> {
    let mut fingerprints: HashMap<String, ()> = HashMap::new();
    let mut duplicates: Vec<&DocumentTransition> = vec![];

    for dt in document_transitions {
        match fingerprints.entry(create_fingerprint(dt)) {
            Entry::Occupied(_) => {
                duplicates.push(dt);
            }
            Entry::Vacant(v) => {
                v.insert(());
            }
        }
    }
    duplicates
}

fn create_fingerprint(document_transition: &DocumentTransition) -> String {
    match document_transition {
        DocumentTransition::Create(ref dt) => fingerprint(&dt.base),
        DocumentTransition::Delete(ref dt) => fingerprint(&dt.base),
        DocumentTransition::Replace(ref dt) => fingerprint(&dt.base),
    }
}
fn fingerprint(document: &DocumentBaseTransition) -> String {
    format!(
        "{}:{}",
        document.data_contract_id.to_string(Encoding::Base58),
        document.document_type
    )
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

        let input: Vec<DocumentTransition> = vec![
            DocumentTransition::Create(dt_create),
            DocumentTransition::Create(dt_create_duplicate),
            DocumentTransition::Replace(dt_replace),
            DocumentTransition::Delete(dt_delete),
        ];

        let duplicates = find_duplicates_by_id(input.iter());
        assert_eq!(duplicates.len(), 1);
    }
}
