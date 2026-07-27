use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use platform_value::Identifier;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::{DocumentTransition, DocumentTransitionV0Methods};

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
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::DocumentCreateTransition;
    use crate::state_transition::batch_transition::batched_transition::DocumentReplaceTransition;
    use crate::state_transition::batch_transition::batched_transition::DocumentDeleteTransition;
    use crate::state_transition::batch_transition::batched_transition::document_replace_transition::DocumentReplaceTransitionV0;

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
                prefunded_voting_balance: Default::default(),
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

    fn make_transition_with(id_byte: u8, type_name: &str, nonce: u64) -> DocumentTransition {
        DocumentTransition::Create(DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::new([id_byte; 32]),
                identity_contract_nonce: nonce,
                document_type_name: type_name.to_string(),
                data_contract_id: Identifier::new([0xAA; 32]),
            }),
            entropy: Default::default(),
            data: Default::default(),
            prefunded_voting_balance: Default::default(),
        }))
    }

    #[test]
    fn test_empty_input_returns_empty_duplicates() {
        let input: Vec<&DocumentTransition> = vec![];
        let duplicates = find_duplicates_by_id(&input);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_single_transition_no_duplicates() {
        let t = make_transition_with(1, "doc", 0);
        let input = vec![&t];
        let duplicates = find_duplicates_by_id(&input);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_no_duplicates_when_all_distinct() {
        let t1 = make_transition_with(1, "a", 0);
        let t2 = make_transition_with(2, "a", 1);
        let t3 = make_transition_with(3, "b", 2);
        let input = vec![&t1, &t2, &t3];
        let duplicates = find_duplicates_by_id(&input);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_same_id_different_type_is_not_a_duplicate() {
        // The fingerprint is (document_type, id). If only the id matches but
        // the type differs, it should NOT count as a duplicate.
        let t1 = make_transition_with(1, "type_a", 0);
        let t2 = make_transition_with(1, "type_b", 1);
        let input = vec![&t1, &t2];
        let duplicates = find_duplicates_by_id(&input);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_same_type_different_id_is_not_a_duplicate() {
        let t1 = make_transition_with(1, "type_a", 0);
        let t2 = make_transition_with(2, "type_a", 1);
        let input = vec![&t1, &t2];
        let duplicates = find_duplicates_by_id(&input);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_three_way_duplicate_pushes_pairs() {
        // Three transitions with the same fingerprint. The current
        // implementation pushes the previously-stored entry AND the new
        // duplicate every time a collision is detected — so we expect
        // (existing, t2) on the second insert, then (existing, t3) on the
        // third. That yields four entries total.
        let t1 = make_transition_with(7, "doc", 0);
        let t2 = make_transition_with(7, "doc", 1);
        let t3 = make_transition_with(7, "doc", 2);
        let input = vec![&t1, &t2, &t3];
        let duplicates = find_duplicates_by_id(&input);
        assert_eq!(duplicates.len(), 4);
    }

    #[test]
    fn test_fingerprint_only_uses_type_and_id_not_nonce() {
        // Differing nonces alone should not prevent a duplicate match.
        let t1 = make_transition_with(5, "x", 100);
        let t2 = make_transition_with(5, "x", 200);
        let input = vec![&t1, &t2];
        let duplicates = find_duplicates_by_id(&input);
        assert_eq!(duplicates.len(), 2);
    }
}
