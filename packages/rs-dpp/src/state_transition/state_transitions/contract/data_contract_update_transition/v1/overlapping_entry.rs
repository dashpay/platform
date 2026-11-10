use crate::consensus::basic::data_contract::{
    DataContractUpdateEntryKind, DataContractUpdateOverlappingEntriesError,
};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
use std::collections::BTreeSet;

impl DataContractUpdateTransitionV1 {
    /// Names the first entry this delta lists in two sections that can not
    /// both apply: a document type or schema definition that is both new
    /// and updated, or a keyword that is both added and removed.
    ///
    /// Basic structure validation rejects such a delta with this error and
    /// proof verification refuses to verify one, so one check answers both.
    ///
    /// Basic structure validation runs before a transition's signature is
    /// checked, on lists as long as a state transition lets them be, so the
    /// work here grows with the size of the delta and not with its square.
    pub fn overlapping_entry(&self) -> Option<DataContractUpdateOverlappingEntriesError> {
        let contract_id = self.data_contract_id;

        if let Some(name) = self
            .updated_document_schemas
            .keys()
            .find(|name| self.new_document_schemas.contains_key(*name))
        {
            return Some(DataContractUpdateOverlappingEntriesError::new(
                contract_id,
                DataContractUpdateEntryKind::DocumentType,
                name.clone(),
            ));
        }

        if let Some(name) = self
            .updated_schema_defs
            .keys()
            .find(|name| self.new_schema_defs.contains_key(*name))
        {
            return Some(DataContractUpdateOverlappingEntriesError::new(
                contract_id,
                DataContractUpdateEntryKind::SchemaDef,
                name.clone(),
            ));
        }

        // The removed keywords are looked up in a set: scanning their list for every
        // added keyword would cost the product of the two lengths, millions of
        // comparisons for lists that fit one transition. The added keywords are still
        // read in order, so the first of them that is also removed is the one named.
        let removed_keywords: BTreeSet<&String> = self.remove_keywords.iter().collect();
        if let Some(keyword) = self
            .add_keywords
            .iter()
            .find(|keyword| removed_keywords.contains(keyword))
        {
            return Some(DataContractUpdateOverlappingEntriesError::new(
                contract_id,
                DataContractUpdateEntryKind::Keyword,
                keyword.clone(),
            ));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::Identifier;

    fn delta(
        add_keywords: Vec<String>,
        remove_keywords: Vec<String>,
    ) -> DataContractUpdateTransitionV1 {
        DataContractUpdateTransitionV1 {
            identity_contract_nonce: 1,
            data_contract_id: Identifier::new([1; 32]),
            owner_id: Identifier::new([2; 32]),
            version: 2,
            config: None,
            updated_schema_defs: Default::default(),
            new_schema_defs: Default::default(),
            updated_document_schemas: Default::default(),
            new_document_schemas: Default::default(),
            new_groups: Default::default(),
            new_tokens: Default::default(),
            add_keywords,
            remove_keywords,
            description: Default::default(),
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
    }

    /// As many keywords of one byte as fit in a state transition, twice over.
    const MANY: usize = 5_000;

    #[test]
    fn should_find_no_overlap_between_long_lists_of_repeated_keywords() {
        let delta = delta(vec!["a".to_string(); MANY], vec!["b".to_string(); MANY]);

        assert_eq!(delta.overlapping_entry(), None);
    }

    #[test]
    fn should_name_the_first_added_keyword_that_is_also_removed() {
        let mut add_keywords = vec!["a".to_string(); MANY];
        add_keywords.push("second".to_string());
        add_keywords.push("first".to_string());
        // The removed list names them in the other order
        let mut remove_keywords = vec!["b".to_string(); MANY];
        remove_keywords.push("first".to_string());
        remove_keywords.push("second".to_string());

        let overlap = delta(add_keywords, remove_keywords)
            .overlapping_entry()
            .expect("expected an overlap");

        assert_eq!(overlap.entry_kind(), DataContractUpdateEntryKind::Keyword);
        assert_eq!(overlap.name(), "second");
    }
}
