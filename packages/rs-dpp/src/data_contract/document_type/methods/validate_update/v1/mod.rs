//! Protocol v14 generation of document type update validation.
//!
//! v0 validated index changes by comparing `IndexLevel` trees whose
//! `level_identifier`s are assigned by an incrementing counter while walking
//! `indices` — a BTreeMap keyed by index NAME. Adding an index whose name
//! sorted before an existing one renumbered every level, so the identifier
//! equality check rejected the update with an opaque "Invalid path", while
//! the semantically identical addition under a late-sorting name passed the
//! tree comparison (and then hard-errored in the JSON-schema compatibility
//! check, which has no rule for the `indices` keyword). Which consensus
//! outcome a contract owner got therefore depended purely on how the new
//! index's name sorted.
//!
//! v1 drops the tree comparison and compares the parsed index definitions
//! by name instead: any added, removed or modified index is rejected with a
//! deterministic `DataContractInvalidIndexDefinitionUpdateError` naming the
//! offending index, independent of name sort order. This does not change
//! which updates are ultimately acceptable — under v0 no index modification
//! could ever pass the full pipeline (whatever survived the tree comparison
//! was always rejected by the `indices` schema-compatibility hard error) —
//! it makes the rejection deterministic, clean, and correctly labeled.

use crate::consensus::basic::data_contract::DataContractInvalidIndexDefinitionUpdateError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DocumentTypeRef<'_> {
    #[inline(always)]
    pub(super) fn validate_update_v1(
        &self,
        new_document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Validate configuration
        let result = self.validate_config(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that index definitions are unchanged
        let result = self.validate_index_definitions_unchanged(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that no byte array property changes its on-disk encoding
        let result = self.validate_byte_array_encoding_stability(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate schema compatibility
        self.validate_schema(new_document_type, platform_version)
    }

    /// Index definitions are immutable once a document type is registered:
    /// Drive lays out the index trees at contract creation and never
    /// backfills them, so an added index would silently miss every
    /// pre-update document and a removed or modified one would orphan
    /// on-disk subtrees. Compare the definitions by index name — the
    /// comparison must not depend on where a changed index's name sorts
    /// relative to the document type's other indexes.
    fn validate_index_definitions_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let old_indexes = self.indexes();
        let new_indexes = new_document_type.indexes();

        for (name, old_index) in old_indexes {
            match new_indexes.get(name) {
                None => {
                    return SimpleConsensusValidationResult::new_with_error(
                        DataContractInvalidIndexDefinitionUpdateError::new(
                            self.name().to_string(),
                            format!("removed index '{name}'"),
                        )
                        .into(),
                    );
                }
                Some(new_index) if new_index != old_index => {
                    return SimpleConsensusValidationResult::new_with_error(
                        DataContractInvalidIndexDefinitionUpdateError::new(
                            self.name().to_string(),
                            format!("changed index '{name}'"),
                        )
                        .into(),
                    );
                }
                _ => {}
            }
        }

        for name in new_indexes.keys() {
            if !old_indexes.contains_key(name) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidIndexDefinitionUpdateError::new(
                        self.name().to_string(),
                        format!("added index '{name}'"),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::DocumentType;
    use assert_matches::assert_matches;
    use platform_value::{platform_value, Identifier, Value};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn doc_type_with_indices(indices: Value, platform_version: &PlatformVersion) -> DocumentType {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                "c": {"type": "string", "position": 2, "maxLength": 60_u32},
            },
            "indices": indices,
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create document type")
    }

    fn old_doc_type(platform_version: &PlatformVersion) -> DocumentType {
        doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        )
    }

    // The v0 regression this generation fixes: the outcome of adding an
    // index must not depend on where its name sorts relative to the
    // document type's existing indexes. Under v0, adding "i" on [a] was
    // rejected with an opaque "Invalid path" (level renumbering) while the
    // semantically identical "z" on [a] passed the tree comparison and
    // hard-errored later in schema compatibility. Under v1 both get the
    // same clean rejection naming the added index.
    #[test]
    fn should_reject_added_index_identically_regardless_of_name_sort_order() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new_early_name = doc_type_with_indices(
            platform_value!([
                {"name": "i", "properties": [{"a": "asc"}]},
                {"name": "j", "properties": [{"c": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let new_late_name = doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
                {"name": "z", "properties": [{"a": "asc"}]},
            ]),
            platform_version,
        );

        let early_result = old
            .as_ref()
            .validate_update(new_early_name.as_ref(), platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            early_result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "added index 'i'"
        );

        let late_result = old
            .as_ref()
            .validate_update(new_late_name.as_ref(), platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            late_result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "added index 'z'"
        );
    }

    // Renaming an index leaves the `IndexLevel` tree unchanged (index names
    // are not part of it), so under v0 a rename either slipped through to a
    // schema-compatibility hard error or — when it shifted the name-order
    // level numbering — was rejected as "Invalid path". Under v1 it is a
    // clean, deterministic rejection.
    #[test]
    fn should_reject_renamed_index_with_clean_error() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        // "j" renamed to "zz" — this also shifts the v0 level numbering
        // because "zz" sorts after "k" while "j" sorted before it.
        let new = doc_type_with_indices(
            platform_value!([
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
                {"name": "zz", "properties": [{"c": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "removed index 'j'"
        );
    }

    #[test]
    fn should_reject_removed_index() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new = doc_type_with_indices(
            platform_value!([
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "removed index 'j'"
        );
    }

    #[test]
    fn should_reject_index_with_added_property() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new = doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}, {"a": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "changed index 'j'"
        );
    }

    // Flipping `unique` leaves the v0 `IndexLevel` subset comparison
    // blind (it never compared terminator info), so v0 let it through to
    // the schema-compatibility hard error. v1 rejects it cleanly.
    #[test]
    fn should_reject_index_with_changed_unique_flag() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new = doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}], "unique": true},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "changed index 'j'"
        );
    }

    #[test]
    fn should_pass_when_indices_are_unchanged() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);
        let new = old_doc_type(platform_version);

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "unchanged document type should be accepted, got {:?}",
            result.errors
        );
    }
}
