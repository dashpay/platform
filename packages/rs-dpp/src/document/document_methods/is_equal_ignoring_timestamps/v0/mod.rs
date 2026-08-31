use crate::document::document_methods::DocumentGetRawForDocumentTypeV0;
use crate::document::DocumentV0Getters;
use platform_value::btreemap_extensions::EqualUnderlyingData;
use platform_value::Value;
use std::collections::BTreeMap;

pub trait DocumentIsEqualIgnoringTimestampsV0:
    DocumentV0Getters + DocumentGetRawForDocumentTypeV0
{
    /// Checks to see if a document is equal without time based fields.
    /// Since these fields are set on the network this function can be useful to make sure that
    /// fields that were supplied have not changed, while ignoring those that are set network side.
    /// Time based fields that are ignored are
    ///     created_at/updated_at
    ///     created_at_block_height/updated_at_block_height
    ///     created_at_core_block_height/updated_at_core_block_height
    fn is_equal_ignoring_time_based_fields_v0(
        &self,
        rhs: &Self,
        also_ignore_fields: Option<Vec<&str>>,
    ) -> bool {
        fn filtered_properties<'a>(
            properties: &'a BTreeMap<String, Value>,
            ignore_fields: &Vec<&str>,
        ) -> BTreeMap<&'a String, &'a Value> {
            properties
                .iter()
                .filter(|(key, _)| !ignore_fields.contains(&key.as_str()))
                .collect()
        }

        let properties_equal = if let Some(fields) = &also_ignore_fields {
            filtered_properties(self.properties(), fields)
                .equal_underlying_data(&filtered_properties(rhs.properties(), fields))
        } else {
            self.properties().equal_underlying_data(rhs.properties())
        };

        self.id() == rhs.id()
            && self.owner_id() == rhs.owner_id()
            && properties_equal
            && self.revision() == rhs.revision()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentV0;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    fn make_base_document() -> DocumentV0 {
        let mut properties = BTreeMap::new();
        properties.insert("name".to_string(), Value::Text("Alice".to_string()));
        properties.insert("score".to_string(), Value::U64(100));

        DocumentV0 {
            contract_version: None,
            id: Identifier::new([1u8; 32]),
            owner_id: Identifier::new([2u8; 32]),
            properties,
            revision: Some(1),
            created_at: Some(1_000_000),
            updated_at: Some(2_000_000),
            transferred_at: Some(3_000_000),
            created_at_block_height: Some(10),
            updated_at_block_height: Some(20),
            transferred_at_block_height: Some(30),
            created_at_core_block_height: Some(5),
            updated_at_core_block_height: Some(6),
            transferred_at_core_block_height: Some(7),
            creator_id: None,
        }
    }

    // ================================================================
    //  Documents with same data but different timestamps are equal
    // ================================================================

    #[test]
    fn equal_documents_with_different_timestamps_returns_true() {
        let doc1 = make_base_document();
        let mut doc2 = make_base_document();

        // Change all time-based fields
        doc2.created_at = Some(9_999_999);
        doc2.updated_at = Some(8_888_888);
        doc2.transferred_at = Some(7_777_777);
        doc2.created_at_block_height = Some(999);
        doc2.updated_at_block_height = Some(888);
        doc2.transferred_at_block_height = Some(777);
        doc2.created_at_core_block_height = Some(99);
        doc2.updated_at_core_block_height = Some(88);
        doc2.transferred_at_core_block_height = Some(77);

        assert!(
            doc1.is_equal_ignoring_time_based_fields_v0(&doc2, None),
            "documents with same id/owner/revision/properties but different timestamps should be equal"
        );
    }

    // ================================================================
    //  Documents with different properties are not equal
    // ================================================================

    #[test]
    fn documents_with_different_properties_returns_false() {
        let doc1 = make_base_document();
        let mut doc2 = make_base_document();

        doc2.properties
            .insert("name".to_string(), Value::Text("Bob".to_string()));

        assert!(
            !doc1.is_equal_ignoring_time_based_fields_v0(&doc2, None),
            "documents with different properties should not be equal"
        );
    }

    // ================================================================
    //  Documents with different IDs are not equal
    // ================================================================

    #[test]
    fn documents_with_different_ids_returns_false() {
        let doc1 = make_base_document();
        let mut doc2 = make_base_document();
        doc2.id = Identifier::new([99u8; 32]);

        assert!(
            !doc1.is_equal_ignoring_time_based_fields_v0(&doc2, None),
            "documents with different IDs should not be equal"
        );
    }

    // ================================================================
    //  Documents with different owner IDs are not equal
    // ================================================================

    #[test]
    fn documents_with_different_owner_ids_returns_false() {
        let doc1 = make_base_document();
        let mut doc2 = make_base_document();
        doc2.owner_id = Identifier::new([99u8; 32]);

        assert!(
            !doc1.is_equal_ignoring_time_based_fields_v0(&doc2, None),
            "documents with different owner IDs should not be equal"
        );
    }

    // ================================================================
    //  Documents with different revisions are not equal
    // ================================================================

    #[test]
    fn documents_with_different_revisions_returns_false() {
        let doc1 = make_base_document();
        let mut doc2 = make_base_document();
        doc2.revision = Some(99);

        assert!(
            !doc1.is_equal_ignoring_time_based_fields_v0(&doc2, None),
            "documents with different revisions should not be equal"
        );
    }

    // ================================================================
    //  also_ignore_fields filters additional properties
    // ================================================================

    #[test]
    fn also_ignore_fields_excludes_specified_properties() {
        let doc1 = make_base_document();
        let mut doc2 = make_base_document();
        // Change a property that we will explicitly ignore
        doc2.properties.insert("score".to_string(), Value::U64(999));

        // Without ignoring, they should differ
        assert!(
            !doc1.is_equal_ignoring_time_based_fields_v0(&doc2, None),
            "should differ when score is changed"
        );

        // With "score" ignored, they should be equal
        assert!(
            doc1.is_equal_ignoring_time_based_fields_v0(&doc2, Some(vec!["score"])),
            "should be equal when score is in the ignore list"
        );
    }

    #[test]
    fn also_ignore_fields_with_multiple_fields() {
        let doc1 = make_base_document();
        let mut doc2 = make_base_document();
        doc2.properties
            .insert("name".to_string(), Value::Text("Bob".to_string()));
        doc2.properties.insert("score".to_string(), Value::U64(999));

        assert!(
            doc1.is_equal_ignoring_time_based_fields_v0(&doc2, Some(vec!["name", "score"])),
            "should be equal when all differing fields are ignored"
        );
    }

    // ================================================================
    //  Empty properties case
    // ================================================================

    #[test]
    fn documents_with_empty_properties_are_equal() {
        let mut doc1 = make_base_document();
        let mut doc2 = make_base_document();
        doc1.properties.clear();
        doc2.properties.clear();
        doc2.created_at = Some(9_999_999);

        assert!(
            doc1.is_equal_ignoring_time_based_fields_v0(&doc2, None),
            "empty-property documents with same ids should be equal ignoring timestamps"
        );
    }

    // ================================================================
    //  also_ignore_fields with a field that doesn't exist in either
    //  document: behaves the same as no ignore list.
    // ================================================================

    #[test]
    fn also_ignore_fields_with_nonexistent_field_has_no_effect() {
        let doc1 = make_base_document();
        let doc2 = make_base_document();
        assert!(doc1.is_equal_ignoring_time_based_fields_v0(
            &doc2,
            Some(vec!["this_field_does_not_exist"])
        ));
    }

    // ================================================================
    //  Empty also_ignore_fields vec: equivalent to None.
    // ================================================================

    #[test]
    fn also_ignore_fields_empty_vec_is_equivalent_to_none() {
        let doc1 = make_base_document();
        let mut doc2 = make_base_document();
        doc2.properties
            .insert("name".to_string(), Value::Text("Bob".to_string()));
        // Empty ignore list means all properties are compared.
        assert!(!doc1.is_equal_ignoring_time_based_fields_v0(&doc2, Some(vec![])));
    }

    // ================================================================
    //  Revision None vs Some is not equal
    // ================================================================

    #[test]
    fn documents_with_none_and_some_revision_are_not_equal() {
        let mut doc1 = make_base_document();
        let doc2 = make_base_document();
        doc1.revision = None;
        assert!(!doc1.is_equal_ignoring_time_based_fields_v0(&doc2, None));
    }

    // ================================================================
    //  Self-equal: same document equals itself (time-based differences
    //  shouldn't matter when both sides are identical).
    // ================================================================

    #[test]
    fn a_document_equals_itself_ignoring_time_based_fields() {
        let doc = make_base_document();
        assert!(doc.is_equal_ignoring_time_based_fields_v0(&doc, None));
    }
}
