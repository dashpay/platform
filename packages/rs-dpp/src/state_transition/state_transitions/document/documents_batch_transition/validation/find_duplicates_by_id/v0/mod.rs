use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};

use platform_value::Identifier;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
struct TransitionFingerprint<'a> {
    document_type: &'a str,
    id: Identifier,
}

impl<'a> From<&'a DocumentTransition> for TransitionFingerprint<'a> {
    fn from(transition: &'a DocumentTransition) -> Self {
        let base = transition.base();

        Self {
            document_type: base.document_type_name(),
            id: base.id(),
        }
    }
}

/// Find the duplicates in the collection of Document Transitions
pub(super) fn find_duplicates_by_id<'a>(
    document_transitions: &'a Vec<&DocumentTransition>,
) -> Vec<&'a DocumentTransition> {
    let mut fingerprints: BTreeMap<TransitionFingerprint, &DocumentTransition> = BTreeMap::new();
    let mut duplicates: Vec<&DocumentTransition> = vec![];

    for transition in document_transitions {
        let fingerprint = (*transition).into();

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

    duplicates
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;
    use crate::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;
    use crate::state_transition::documents_batch_transition::document_transition::DocumentCreateTransition;
    use crate::state_transition::documents_batch_transition::document_transition::DocumentReplaceTransition;
    use crate::state_transition::documents_batch_transition::document_transition::DocumentDeleteTransition;
    use crate::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;

    #[test]
    fn test_duplicates() {
        let create_transition =
            DocumentTransition::Create(DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                    id: Identifier::random(),
                    identity_contract_nonce: 0,
                    document_type_name: "a".to_string(),
                    data_contract_id: Identifier::random(),
                }),
                entropy: Default::default(),
                data: Default::default(),
            }));

        let create_transition_duplicate = create_transition.clone();

        let replace_transition = DocumentTransition::Replace(DocumentReplaceTransition::V0(
            DocumentReplaceTransitionV0 {
                base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                    id: Identifier::random(),
                    identity_contract_nonce: 1,
                    document_type_name: "a".to_string(),
                    data_contract_id: Identifier::random(),
                }),
                revision: Default::default(),
                data: Default::default(),
            },
        ));

        let delete_transition =
            DocumentTransition::Delete(DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
                base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                    id: Identifier::random(),
                    identity_contract_nonce: 2,
                    document_type_name: "c".to_string(),
                    data_contract_id: Identifier::random(),
                }),
            }));

        let input = vec![
            &create_transition,
            &create_transition_duplicate,
            &replace_transition,
            &delete_transition,
        ];

        let duplicates = find_duplicates_by_id(&input);

        assert_eq!(duplicates.len(), 2);
    }
}
