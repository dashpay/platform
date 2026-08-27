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

use crate::consensus::basic::data_contract::{
    DataContractInvalidIndexDefinitionUpdateError, DataContractInvalidRequiredFieldsUpdateError,
};
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
        new_contract_version: u32,
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

        // Validate required-field changes (the schema compatibility differ
        // has the top-level `required` key stripped, so this is the only
        // place top-level requiredness changes are judged)
        let result = self.validate_required_fields_update(new_document_type, new_contract_version);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate schema compatibility
        self.validate_schema(new_document_type, platform_version)
    }

    /// Top-level requiredness may only change in one way: a brand-new
    /// property may be added as required when it is annotated with
    /// `requiredSince` equal to the contract version this update creates.
    /// Everything else is frozen: requiredness is baked into the document
    /// wire format (required properties serialize without a presence flag),
    /// and the per-document contract-version stamp resolves layouts from the
    /// latest schema alone only if annotations never change retroactively.
    ///
    /// Nested (dotted) required paths and the `requiredSince` keyword on
    /// existing properties stay frozen by the schema compatibility differ;
    /// this check judges the top-level `required` key, which is stripped
    /// from the diff exactly like `indices`.
    fn validate_required_fields_update(
        &self,
        new_document_type: DocumentTypeRef,
        new_contract_version: u32,
    ) -> SimpleConsensusValidationResult {
        let old_required = self.required_fields();
        let new_required = new_document_type.required_fields();

        for name in old_required {
            // Nested paths are governed by the schema compatibility differ
            if name.contains('.') {
                continue;
            }
            if !new_required.contains(name) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!("removed required field '{name}'"),
                    )
                    .into(),
                );
            }
        }

        for name in new_required {
            if name.contains('.') || old_required.contains(name) {
                continue;
            }
            if name.starts_with('$') {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!("system field '{name}' cannot become required"),
                    )
                    .into(),
                );
            }
            if self.properties().contains_key(name) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!("existing property '{name}' cannot become required"),
                    )
                    .into(),
                );
            }
            let Some(new_property) = new_document_type.properties().get(name) else {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!("added required field '{name}' references an unknown property"),
                    )
                    .into(),
                );
            };
            if new_property.required_since != Some(new_contract_version) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!(
                            "new required field '{name}' must carry requiredSince {new_contract_version}, the contract version this update creates"
                        ),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
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
            .validate_update(new_early_name.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            early_result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "added index 'i'"
        );

        let late_result = old
            .as_ref()
            .validate_update(new_late_name.as_ref(), 2, platform_version)
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
            .validate_update(new.as_ref(), 2, platform_version)
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
            .validate_update(new.as_ref(), 2, platform_version)
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
            .validate_update(new.as_ref(), 2, platform_version)
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
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "changed index 'j'"
        );
    }

    // Reordering the `indices` array without changing the definition set is
    // a semantic no-op (indices are keyed by name), so the name-keyed
    // comparison passes — and the schema-compatibility check must not trip
    // over the surviving `/indices` JSON diff. Under protocol v13 that diff
    // hit the unsupported-keyword hard error (an internal error, not a
    // consensus-invalid result); at v14 `validate_schema_compatibility` v1
    // strips `indices` before diffing and the update validates cleanly.
    #[test]
    fn should_pass_when_indices_are_reordered_without_changes() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new = doc_type_with_indices(
            platform_value!([
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
                {"name": "j", "properties": [{"c": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "a reorder-only indices update should be accepted, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_pass_when_indices_are_unchanged() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);
        let new = old_doc_type(platform_version);

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "unchanged document type should be accepted, got {:?}",
            result.errors
        );
    }

    // Ranked aggregate indexes (protocol v14 grammar) are covered by the
    // same name-keyed definition comparison as every other index flag:
    // toggling a ranking axis after creation changes the on-disk tree
    // variant, so it must be rejected. The ranking axes are index-level,
    // so `validate_config` — which covers the *doctype*-level count / sum
    // flags — is deliberately not where they are enforced. These tests
    // exercise the PUBLIC dispatcher so that routing is pinned, not just
    // the helper in isolation; they live here rather than in v0 because
    // the ranked grammar only exists at protocol v14, where
    // validate_update dispatches to v1.
    mod validate_update_ranked_indices {
        use super::*;

        /// `review` doctype, one averageable index over `restaurantId`, with
        /// `rankedAverageable` set to the supplied value.
        fn document_type_with_ranked_index(
            ranked_averageable: bool,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    // 32 rather than the generic 63-character index limit:
                    // an index declaring a ranking axis bounds its group key
                    // more tightly (59 characters on the Avg axis), and both
                    // halves of these tests have to build the same doctype
                    // shape with only `rankedAverageable` differing.
                    "restaurantId": {
                        "type": "string",
                        "maxLength": 32,
                        "position": 0,
                    },
                    "grade": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100,
                        "position": 1,
                    },
                },
                "required": ["restaurantId", "grade"],
                "additionalProperties": false,
                "indices": [{
                    "name": "byRestaurant",
                    "properties": [{ "restaurantId": "asc" }],
                    "averageable": "grade",
                    "rangeAverageable": true,
                    "rankedAverageable": ranked_averageable,
                }],
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            DocumentType::try_from_schema(
                Identifier::random(),
                1,
                config.version(),
                "review",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create document type")
        }

        #[test]
        fn should_return_invalid_result_when_ranked_averageable_is_changed() {
            let platform_version = PlatformVersion::latest();
            let old = document_type_with_ranked_index(false, platform_version);
            let new = document_type_with_ranked_index(true, platform_version);

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
                )] if e.index_path() == "changed index 'byRestaurant'"
            );
        }

        #[test]
        fn should_pass_when_ranked_averageable_is_unchanged() {
            let platform_version = PlatformVersion::latest();
            let old = document_type_with_ranked_index(true, platform_version);
            let new = document_type_with_ranked_index(true, platform_version);

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert!(
                result.is_valid(),
                "an unchanged ranked index must not be rejected, got {:?}",
                result.errors
            );
        }
    }

    // ================================================================
    //  Required-field updates (`requiredSince`)
    // ================================================================

    mod required_fields_update {
        use super::*;

        fn doc_type_with(
            properties: Value,
            required: Value,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": properties,
                "required": required,
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
            doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                }),
                platform_value!(["a"]),
                platform_version,
            )
        }

        #[test]
        fn should_allow_adding_new_required_property_with_correct_required_since() {
            let platform_version = PlatformVersion::latest();

            let old = old_doc_type(platform_version);
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32, "requiredSince": 2},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert!(
                result.is_valid(),
                "a new required property annotated with the version this \
                 update creates must be accepted, got {:?}",
                result.errors
            );
        }

        #[test]
        fn should_reject_new_required_property_with_retroactive_required_since() {
            let platform_version = PlatformVersion::latest();

            let old = old_doc_type(platform_version);
            // Contract moving to version 3, but the annotation claims 2:
            // documents stamped 2 would misparse
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32, "requiredSince": 2},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 3, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
                )] if e.details().contains("must carry requiredSince 3")
            );
        }

        #[test]
        fn should_reject_new_required_property_without_required_since() {
            let platform_version = PlatformVersion::latest();

            let old = old_doc_type(platform_version);
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
                )] if e.details().contains("must carry requiredSince 2")
            );
        }

        #[test]
        fn should_reject_promoting_existing_property_to_required() {
            let platform_version = PlatformVersion::latest();

            let old = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a"]),
                platform_version,
            );
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
                )] if e.details() == "existing property 'b' cannot become required"
            );
        }

        #[test]
        fn should_reject_removing_required_field() {
            let platform_version = PlatformVersion::latest();

            let old = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
                )] if e.details() == "removed required field 'b'"
            );
        }

        #[test]
        fn should_reject_mutating_required_since_on_existing_property() {
            let platform_version = PlatformVersion::latest();

            // The property was added as required at version 2; a later
            // update must not move the annotation. This is caught by the
            // compatibility differ's frozen `requiredSince` rule.
            let old = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32, "requiredSince": 2},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32, "requiredSince": 3},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 3, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IncompatibleDocumentTypeSchemaError(e)
                )] if e.operation() == "replace" && e.property_path() == "/properties/b/requiredSince"
            );
        }
    }
}
