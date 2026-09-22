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
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::common::UpdateValidationOptions;

impl DocumentTypeRef<'_> {
    #[inline(always)]
    pub(super) fn validate_update_v1(
        &self,
        new_document_type: DocumentTypeRef,
        new_contract_version: u32,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Legacy keep-history types advertised deletes that Drive never allowed.
        // Permit only true -> false for that flag while keeping history enabled.
        // Every other config and schema check still runs, and v0 stays immutable.
        let options = UpdateValidationOptions {
            allow_history_delete_repair: self.documents_keep_history()
                && new_document_type.documents_keep_history()
                && self.documents_can_be_deleted()
                && !new_document_type.documents_can_be_deleted(),
        };
        let result = self.validate_config_with_options(new_document_type, &options);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that the type keeps whether moderators may delete its
        // documents (a generation 1 rule: the keyword arrives with protocol
        // version 14, and the shared config checks above also serve v0)
        let result = self.validate_can_be_deleted_by_moderators_unchanged(new_document_type);

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

        // Validate that no integer property changes its width or signedness
        let result = self.validate_integer_encoding_stability(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that no typed array changes how its elements are encoded
        let result = self.validate_typed_array_element_encoding_stability(new_document_type);

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

        // Validate immutable-property changes (the schema compatibility
        // differ has the top-level `immutable` key stripped, so this is the
        // only place those changes are judged)
        let result = self.validate_immutable_fields_update(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that the action fees are unchanged
        let result = self.validate_action_fees_unchanged(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate schema compatibility
        self.validate_schema_with_options(new_document_type, platform_version, &options)
    }

    /// An integer property is stored at the width and signedness of its type,
    /// in the document and in every index key on it, and the type comes from
    /// the property's `minimum` and `maximum`, or from its `enum` values when
    /// it has no bounds (with `sizedIntegerTypes` on; off, every integer is an
    /// i64). The schema compatibility rules allow each change that moves it:
    /// raising `maximum`, lowering `minimum`, removing either, adding `enum`
    /// values, and so does turning `sizedIntegerTypes` on. Documents already
    /// stored then no longer decode, or decode to other values, and their
    /// index entries sit under keys of the old width. So the type is held
    /// here, as `validate_byte_array_encoding_stability` holds a byte array
    /// property's; a bound change that keeps the type is still allowed. The
    /// signedness is held with the width, even where the old bounds keep every
    /// stored value readable both ways (a u8 capped at 100 read as an i8):
    /// nothing bounds a u64 that becomes an i64, and one comparison is the
    /// whole rule. A change to a non-integer type is the schema compatibility
    /// check's to refuse.
    fn validate_integer_encoding_stability(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let new_properties = new_document_type.flattened_properties();

        for (path, old_property) in self.flattened_properties() {
            if !old_property.property_type.is_integer() {
                continue;
            }
            let Some(new_property) = new_properties.get(path) else {
                continue;
            };
            if !new_property.property_type.is_integer() {
                continue;
            }

            let old_encoding = old_property.property_type.stored_encoding();
            let new_encoding = new_property.property_type.stored_encoding();
            if old_encoding != new_encoding {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.data_contract_id(),
                        self.name(),
                        format!(
                            "document type can not change the integer encoding of property \
                             '{}': its values are stored as {} and would be read as {}",
                            path, old_encoding, new_encoding,
                        ),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// A typed array stores each element exactly as a required scalar property
    /// of its element type is stored, so an update that changes how an
    /// element encodes would misread every element already stored: an
    /// integer element whose width changes (its bounds or `enum` choose it),
    /// or a byte array element that turns from fixed-size (raw) to variable
    /// (length-prefixed) or to another fixed size. The schema compatibility
    /// rules allow the changes that do this (raising `maximum`, widening
    /// `maxItems`), so the element encoding is held here, as
    /// `validate_byte_array_encoding_stability` holds a byte array
    /// property's. Every other element change, a longer `maxLength` included,
    /// leaves the encoding as it is.
    fn validate_typed_array_element_encoding_stability(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let new_properties = new_document_type.flattened_properties();

        for (path, old_property) in self.flattened_properties() {
            let DocumentPropertyType::TypedArray(old_array) = &old_property.property_type else {
                continue;
            };
            let Some(new_property) = new_properties.get(path) else {
                continue;
            };
            let DocumentPropertyType::TypedArray(new_array) = &new_property.property_type else {
                continue;
            };

            let old_encoding = old_array.item_type.stored_encoding();
            let new_encoding = new_array.item_type.stored_encoding();
            if old_encoding != new_encoding {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.data_contract_id(),
                        self.name(),
                        format!(
                            "document type can not change the element encoding of typed array \
                             property '{}': its elements are stored as {} and would be read as \
                             {}",
                            path, old_encoding, new_encoding,
                        ),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// The action fees of a document type are fixed when it is published: an
    /// update may not add, change or remove them, nor switch their pricing.
    /// A document type added by the update is not judged here and may declare
    /// its own. A transition names the fee it agrees to pay (its action fee
    /// agreement), so lifting this later cannot make a signed transition pay
    /// a fee its signer never saw.
    fn validate_action_fees_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        if self.action_fees() == new_document_type.action_fees() {
            return SimpleConsensusValidationResult::new();
        }
        let change = match (self.action_fees(), new_document_type.action_fees()) {
            (None, Some(_)) => "add",
            (Some(_), None) => "remove",
            _ => "change",
        };
        SimpleConsensusValidationResult::new_with_error(
            DocumentTypeUpdateError::new(
                self.data_contract_id(),
                self.name(),
                format!(
                    "document type can not {change} its action fees: they are fixed when the \
                     document type is published"
                ),
            )
            .into(),
        )
    }

    /// Whether moderators may delete documents of a type is fixed when the
    /// type is created: whoever wrote a document knows from the type's first
    /// version who may take it down, and a type that is the target of a
    /// permanentDocument reference was admitted as one nobody can delete.
    /// A document type added by an update declares the flag freely.
    fn validate_can_be_deleted_by_moderators_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        if new_document_type.documents_can_be_deleted_by_moderators()
            == self.documents_can_be_deleted_by_moderators()
        {
            // The window the moderators have is fixed with the flag: a longer one would
            // reopen documents that had settled, and one rule for both directions keeps
            // what an author was told when they wrote.
            let (old_window, new_window) = (
                self.documents_can_be_deleted_by_moderators_for(),
                new_document_type.documents_can_be_deleted_by_moderators_for(),
            );
            if old_window == new_window {
                return SimpleConsensusValidationResult::new();
            }
            let seconds = |window: Option<u32>| {
                window.map_or("no limit".to_string(), |seconds| {
                    format!("{seconds} seconds")
                })
            };
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change for how long after a document's last modification moderators can delete it: changing from {} to {}",
                        seconds(old_window),
                        seconds(new_window)
                    ),
                )
                .into(),
            );
        }
        SimpleConsensusValidationResult::new_with_error(
            DocumentTypeUpdateError::new(
                self.data_contract_id(),
                self.name(),
                format!(
                    "document type can not change whether its documents can be deleted by moderators: changing from {} to {}",
                    self.documents_can_be_deleted_by_moderators(),
                    new_document_type.documents_can_be_deleted_by_moderators()
                ),
            )
            .into(),
        )
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

    /// The `immutable` property list may only grow. Removing an entry would
    /// let a later replace change a property that documents were created
    /// under the promise of never changing; adding one only narrows what
    /// future replaces may touch and invalidates no stored document (the
    /// parser has already checked that every new entry names a top-level
    /// property of the new type).
    ///
    /// `immutableAllowSetting` may only shrink, with one exception: a
    /// property that becomes immutable in this very update may arrive with
    /// the allowance. Dropping an entry is a tightening (a still-absent
    /// property can no longer be set). Adding one for a property that was
    /// already immutable would relax a promise the documents were created
    /// under, exactly like removing it from `immutable`.
    ///
    /// Judged here because both top-level keys are stripped from the schema
    /// compatibility diff, exactly like `indices` and `required`.
    fn validate_immutable_fields_update(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let old_immutable = self.immutable_fields();
        let new_immutable = new_document_type.immutable_fields();

        for property in old_immutable {
            if !new_immutable.contains(property) {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.data_contract_id(),
                        self.name(),
                        format!(
                            "document type can not remove immutable property '{property}': the \
                             immutable list may only grow"
                        ),
                    )
                    .into(),
                );
            }
        }

        let old_allow_setting = self.immutable_fields_allow_setting();
        for property in new_document_type.immutable_fields_allow_setting() {
            if !old_allow_setting.contains(property) && old_immutable.contains(property) {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.data_contract_id(),
                        self.name(),
                        format!(
                            "document type can not allow setting immutable property '{property}' \
                             once it is already immutable: only a property that becomes \
                             immutable in this update may be listed in immutableAllowSetting"
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
    use crate::consensus::state::state_error::StateError;
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

    /// A mutable document type with three string properties and the given
    /// `immutable` list.
    fn doc_type_with_immutable(
        immutable: Value,
        platform_version: &PlatformVersion,
    ) -> DocumentType {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                "c": {"type": "string", "position": 2, "maxLength": 60_u32},
            },
            "immutable": immutable,
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
            true,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create document type")
    }

    // The `immutable` list may only grow: removing an entry would let a
    // later replace change a property documents were created under the
    // promise of never changing.
    #[test]
    fn should_return_invalid_result_when_can_be_deleted_by_moderators_is_changed() {
        use crate::data_contract::config::moderation::{
            ContractModerationConfig, ContractModerators,
        };

        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();

        let schema = |deletable_by_moderators: bool| {
            platform_value!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "additionalProperties": false,
                "canBeDeletedByModerators": deletable_by_moderators,
            })
        };

        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));

        let make_document_type = |schema: platform_value::Value| {
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "post",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };

        // Neither direction is allowed: authors keep the rules they wrote under, and a
        // permanentDocument reference was admitted against a type nobody can delete.
        for (old_flag, new_flag) in [(false, true), (true, false)] {
            let old_document_type = make_document_type(schema(old_flag));
            let new_document_type = make_document_type(schema(new_flag));

            // The whole generation 1 pipeline: the rule answers before the schema
            // compatibility differ, which has no rule for the keyword.
            let result = old_document_type
                .as_ref()
                .validate_update(new_document_type.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            let expected = format!(
                "document type can not change whether its documents can be deleted by moderators: changing from {} to {}",
                old_flag, new_flag
            );
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == expected
            );
        }
    }

    #[test]
    fn should_return_invalid_result_when_the_moderators_window_is_changed() {
        use crate::data_contract::config::moderation::{
            ContractModerationConfig, ContractModerators,
        };

        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        let make_document_type = |window: Option<u32>| {
            let mut schema = platform_value!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "required": ["$updatedAt"],
                "additionalProperties": false,
                "canBeDeletedByModerators": true,
            });
            if let Some(seconds) = window {
                schema
                    .insert("canBeDeletedByModeratorsFor".to_string(), seconds.into())
                    .expect("expected to set the window");
            }
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "post",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };

        // Longer would reopen documents that had settled; shorter, given or taken away, would
        // still change what an author was told. One rule for every direction.
        for (old_window, new_window, from, to) in [
            (Some(86400), Some(172800), "86400 seconds", "172800 seconds"),
            (Some(86400), Some(3600), "86400 seconds", "3600 seconds"),
            (Some(86400), None, "86400 seconds", "no limit"),
            (None, Some(86400), "no limit", "86400 seconds"),
        ] {
            let result = make_document_type(old_window)
                .as_ref()
                .validate_update(make_document_type(new_window).as_ref(), 2, platform_version)
                .expect("validate_update should not error");
            let expected = format!(
                "document type can not change for how long after a document's last modification moderators can delete it: changing from {from} to {to}"
            );
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message() == expected
            );
        }

        // Unchanged, it passes.
        let result = make_document_type(Some(86400))
            .as_ref()
            .validate_update(
                make_document_type(Some(86400)).as_ref(),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_reject_removing_an_immutable_property() {
        let platform_version = PlatformVersion::latest();

        let old = doc_type_with_immutable(platform_value!(["a", "b"]), platform_version);
        let new = doc_type_with_immutable(platform_value!(["a"]), platform_version);

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                if e.additional_message()
                    == "document type can not remove immutable property 'b': the immutable list may only grow"
        );
    }

    // Adding an entry narrows what future replaces may touch and invalidates
    // no stored document. This runs the whole v1 pipeline, so it also pins
    // that the compatibility differ ignores the top-level `immutable` key
    // instead of hard-erroring on a keyword it has no rule for.
    #[test]
    fn should_accept_adding_an_immutable_property() {
        let platform_version = PlatformVersion::latest();

        let old = doc_type_with_immutable(platform_value!(["a"]), platform_version);
        let new = doc_type_with_immutable(platform_value!(["a", "c"]), platform_version);

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "growing the immutable list must be accepted, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_accept_an_unchanged_immutable_list() {
        let platform_version = PlatformVersion::latest();

        let old = doc_type_with_immutable(platform_value!(["a", "b"]), platform_version);
        let new = doc_type_with_immutable(platform_value!(["b", "a"]), platform_version);

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "an unchanged (reordered) immutable list must be accepted, got {:?}",
            result.errors
        );
    }

    /// A document type with one string property and, when given, `actionFees`.
    fn doc_type_with_action_fees(
        action_fees: Option<Value>,
        platform_version: &PlatformVersion,
    ) -> DocumentType {
        let mut schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
            },
            "additionalProperties": false,
        });
        if let Some(action_fees) = action_fees {
            schema
                .insert("actionFees".to_string(), action_fees)
                .expect("expected to set the action fees");
        }
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
            true,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create document type")
    }

    // The fees of a published document type do not change yet, although a transition names
    // the fee it agrees to pay.
    #[test]
    fn should_reject_adding_changing_or_removing_action_fees() {
        let platform_version = PlatformVersion::latest();
        let free = doc_type_with_action_fees(None, platform_version);
        let priced = doc_type_with_action_fees(
            Some(platform_value!({"create": {"owner": 10_u64}})),
            platform_version,
        );
        let repriced = doc_type_with_action_fees(
            Some(platform_value!({"create": {"owner": 11_u64}})),
            platform_version,
        );
        let fixed = doc_type_with_action_fees(
            Some(platform_value!({"pricing": "fixed", "create": {"owner": 10_u64}})),
            platform_version,
        );

        for (old, new, change) in [
            (&free, &priced, "add"),
            (&priced, &free, "remove"),
            (&priced, &repriced, "change"),
            (&priced, &fixed, "change"),
        ] {
            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("expected the update to be judged");
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message().contains(&format!("can not {change} its action fees"))
            );
        }
    }

    #[test]
    fn should_accept_unchanged_action_fees() {
        let platform_version = PlatformVersion::latest();
        let priced = doc_type_with_action_fees(
            Some(platform_value!({"create": {"owner": 10_u64, "moderators": 3_u64}})),
            platform_version,
        );
        let result = priced
            .as_ref()
            .validate_update(priced.as_ref(), 2, platform_version)
            .expect("expected the update to be judged");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    /// Like `doc_type_with_immutable`, with an `immutableAllowSetting` list
    /// as well.
    fn doc_type_with_immutable_lists(
        immutable: Value,
        allow_setting: Value,
        platform_version: &PlatformVersion,
    ) -> DocumentType {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                "c": {"type": "string", "position": 2, "maxLength": 60_u32},
            },
            "immutable": immutable,
            "immutableAllowSetting": allow_setting,
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
            true,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create document type")
    }

    // Dropping an allow-setting entry only tightens: a still-absent property
    // can no longer be set.
    #[test]
    fn should_accept_removing_an_allow_setting_entry() {
        let platform_version = PlatformVersion::latest();

        let old = doc_type_with_immutable_lists(
            platform_value!(["a", "b"]),
            platform_value!(["b"]),
            platform_version,
        );
        let new = doc_type_with_immutable_lists(
            platform_value!(["a", "b"]),
            platform_value!([]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "dropping an allow-setting entry must be accepted, got {:?}",
            result.errors
        );
    }

    // An already-immutable property cannot start allowing a set: that would
    // relax the promise its documents were created under.
    #[test]
    fn should_reject_allowing_setting_of_an_already_immutable_property() {
        let platform_version = PlatformVersion::latest();

        let old = doc_type_with_immutable_lists(
            platform_value!(["a", "b"]),
            platform_value!([]),
            platform_version,
        );
        let new = doc_type_with_immutable_lists(
            platform_value!(["a", "b"]),
            platform_value!(["b"]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                if e.additional_message().starts_with(
                    "document type can not allow setting immutable property 'b' once it is already immutable"
                )
        );
    }

    // A property that becomes immutable in this update may arrive with the
    // allowance: nothing was promised about it before.
    #[test]
    fn should_accept_a_newly_immutable_property_that_allows_setting() {
        let platform_version = PlatformVersion::latest();

        let old = doc_type_with_immutable_lists(
            platform_value!(["a"]),
            platform_value!([]),
            platform_version,
        );
        let new = doc_type_with_immutable_lists(
            platform_value!(["a", "c"]),
            platform_value!(["c"]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "a newly immutable property may allow setting, got {:?}",
            result.errors
        );
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

    // ================================================================
    //  Typed array element encoding
    // ================================================================

    mod typed_array_element_encoding {
        use super::*;

        /// A document type whose one property is a typed array with the
        /// given `items` and `maxItems`.
        fn doc_type_with_list(
            items: Value,
            max_items: u16,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "list": {
                        "type": "array",
                        "maxItems": max_items,
                        "items": items,
                        "position": 0
                    },
                },
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
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create document type")
        }

        /// The schema compatibility rules allow each of these changes, but
        /// each changes how an element is written, so the elements already
        /// stored would be misread.
        #[test]
        fn should_reject_an_update_that_changes_how_typed_array_elements_are_encoded() {
            let platform_version = PlatformVersion::latest();

            for (old_items, new_items, old_encoding, new_encoding) in [
                // Raising the maximum across a width boundary widens the
                // element from one byte to two
                (
                    platform_value!({ "type": "integer", "minimum": 0, "maximum": 100 }),
                    platform_value!({ "type": "integer", "minimum": 0, "maximum": 1000 }),
                    "u8",
                    "u16",
                ),
                // So does adding an enum value past what a byte holds
                (
                    platform_value!({ "type": "integer", "enum": [1, 2, 3] }),
                    platform_value!({ "type": "integer", "enum": [1, 2, 3, 300] }),
                    "u8",
                    "u16",
                ),
                // A byte array element whose size stops being pinned gains a
                // length prefix
                (
                    platform_value!({
                        "type": "array", "byteArray": true, "minItems": 20, "maxItems": 20
                    }),
                    platform_value!({
                        "type": "array", "byteArray": true, "minItems": 20, "maxItems": 32
                    }),
                    "a fixed 20-byte array",
                    "a length-prefixed byte array",
                ),
            ] {
                let old = doc_type_with_list(old_items.clone(), 8, platform_version);
                let new = doc_type_with_list(new_items.clone(), 8, platform_version);

                let result = old
                    .as_ref()
                    .validate_update(new.as_ref(), 2, platform_version)
                    .expect("validate_update should not error");

                let expected = format!(
                    "document type can not change the element encoding of typed array property \
                     'list': its elements are stored as {old_encoding} and would be read as \
                     {new_encoding}"
                );
                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                        if e.additional_message() == expected,
                    "{old_items:?} -> {new_items:?}: {:?}",
                    result.errors
                );
            }
        }

        /// Longer strings, more elements, and a raised maximum that stays
        /// within the element's width leave every stored element readable.
        #[test]
        fn should_accept_an_update_that_keeps_how_typed_array_elements_are_encoded() {
            let platform_version = PlatformVersion::latest();

            for (old_items, old_max_items, new_items, new_max_items) in [
                (
                    platform_value!({ "type": "string", "maxLength": 20 }),
                    8,
                    platform_value!({ "type": "string", "maxLength": 40 }),
                    8,
                ),
                (
                    platform_value!({
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier"
                    }),
                    8,
                    platform_value!({
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier"
                    }),
                    64,
                ),
                (
                    platform_value!({ "type": "integer", "minimum": 0, "maximum": 100 }),
                    8,
                    platform_value!({ "type": "integer", "minimum": 0, "maximum": 200 }),
                    8,
                ),
            ] {
                let old = doc_type_with_list(old_items.clone(), old_max_items, platform_version);
                let new = doc_type_with_list(new_items.clone(), new_max_items, platform_version);

                let result = old
                    .as_ref()
                    .validate_update(new.as_ref(), 2, platform_version)
                    .expect("validate_update should not error");

                assert!(
                    result.is_valid(),
                    "{old_items:?} ({old_max_items}) -> {new_items:?} ({new_max_items}): {:?}",
                    result.errors
                );
            }
        }
    }

    // ================================================================
    //  Integer encoding (width and signedness)
    // ================================================================

    mod integer_encoding_update {
        use super::*;
        use crate::data_contract::config::v1::DataContractConfigSettersV1;
        use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use crate::data_contract::document_type::DocumentPropertyType;
        use crate::validation::SimpleConsensusValidationResult;
        use std::io::BufReader;

        fn doc_type_with(
            properties: Value,
            sized_integer_types: bool,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": properties,
                "additionalProperties": false,
            });
            let mut config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");
            config.set_sized_integer_types_enabled(sized_integer_types);
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

        /// A document type whose one property, `score`, has the given schema.
        fn doc_type_with_score(score: Value, platform_version: &PlatformVersion) -> DocumentType {
            doc_type_with(platform_value!({ "score": score }), true, platform_version)
        }

        fn validate_update(
            old: &DocumentType,
            new: &DocumentType,
            platform_version: &PlatformVersion,
        ) -> SimpleConsensusValidationResult {
            old.as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error")
        }

        fn assert_rejected(
            result: SimpleConsensusValidationResult,
            path: &str,
            old: &str,
            new: &str,
        ) {
            let expected =
                format!("'{path}': its values are stored as {old} and would be read as {new}");
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message().contains(&expected),
                "expected the integer encoding change {old} -> {new} of '{path}' to be refused"
            );
        }

        #[test]
        fn should_not_read_a_stored_u8_back_as_the_u16_a_raised_maximum_gives() {
            // Why the type is held: the bounds choose it, and a value stored
            // at the old width does not read back at the new one.
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 1000, "position": 0}),
                platform_version,
            );
            let old_type = &old.flattened_properties()["score"].property_type;
            let new_type = &new.flattened_properties()["score"].property_type;
            assert_eq!(old_type, &DocumentPropertyType::U8);
            assert_eq!(new_type, &DocumentPropertyType::U16);

            let stored = old_type
                .encode_value_ref_with_size(&Value::U8(7), true)
                .expect("should encode a u8");
            assert_eq!(stored, vec![7]);

            let read_back =
                new_type.read_optionally_from(&mut BufReader::new(stored.as_slice()), true);
            assert!(
                read_back.is_err(),
                "a stored u8 must not read back as a u16, got {read_back:?}"
            );
        }

        #[test]
        fn should_reject_raising_maximum_past_the_width_of_the_type() {
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 1000, "position": 0}),
                platform_version,
            );

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "u8",
                "u16",
            );
        }

        #[test]
        fn should_reject_lowering_minimum_below_zero() {
            // The same width with the other signedness: a stored u64 above
            // i64::MAX would read back negative
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": -1, "position": 0}),
                platform_version,
            );

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "u64",
                "i64",
            );
        }

        #[test]
        fn should_reject_removing_the_bounds() {
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "position": 0}),
                platform_version,
            );

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "u8",
                "i64",
            );
        }

        #[test]
        fn should_reject_adding_an_enum_value_past_the_width_of_the_type() {
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "enum": [1, 2, 3], "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "enum": [1, 2, 3, 300], "position": 0}),
                platform_version,
            );

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "u8",
                "u16",
            );
        }

        #[test]
        fn should_reject_a_width_change_of_a_nested_integer_property() {
            let platform_version = PlatformVersion::latest();
            let stats = |maximum: u32| {
                platform_value!({
                    "stats": {
                        "type": "object",
                        "position": 0,
                        "properties": {
                            "level": {"type": "integer", "minimum": 0, "maximum": maximum, "position": 0},
                        },
                        "additionalProperties": false,
                    },
                })
            };
            let old = doc_type_with(stats(100), true, platform_version);
            let new = doc_type_with(stats(1000), true, platform_version);

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "stats.level",
                "u8",
                "u16",
            );
        }

        #[test]
        fn should_reject_turning_sized_integer_types_on() {
            // Off, every integer is an i64; on, the bounds make this one a u8.
            // The contract config check only refuses turning them off.
            let platform_version = PlatformVersion::latest();
            let properties = platform_value!({
                "score": {"type": "integer", "minimum": 0, "maximum": 100, "position": 0},
            });
            let old = doc_type_with(properties.clone(), false, platform_version);
            let new = doc_type_with(properties, true, platform_version);

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "i64",
                "u8",
            );
        }

        #[test]
        fn should_accept_a_bound_change_that_keeps_the_type() {
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 200, "position": 0}),
                platform_version,
            );

            let result = validate_update(&old, &new, platform_version);
            assert!(
                result.is_valid(),
                "a u8 that stays a u8 must be accepted, got {:?}",
                result.errors
            );
        }

        #[test]
        fn should_accept_raising_maximum_without_sized_integer_types() {
            // Without sized integer types every integer is an i64 whatever
            // its bounds, so raising one changes nothing stored
            let platform_version = PlatformVersion::latest();
            let score = |maximum: u32| {
                platform_value!({
                    "score": {"type": "integer", "minimum": 0, "maximum": maximum, "position": 0},
                })
            };
            let old = doc_type_with(score(100), false, platform_version);
            let new = doc_type_with(score(1000), false, platform_version);

            let result = validate_update(&old, &new, platform_version);
            assert!(
                result.is_valid(),
                "an i64 that stays an i64 must be accepted, got {:?}",
                result.errors
            );
        }

        #[test]
        fn should_still_accept_a_width_change_at_protocol_version_13() {
            // validate_update v0 is frozen for replay of protocol versions up
            // to 13, which let the width move
            let platform_version =
                PlatformVersion::get(13).expect("protocol version 13 must exist");
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 1000, "position": 0}),
                platform_version,
            );

            let result = validate_update(&old, &new, platform_version);
            assert!(
                result.is_valid(),
                "protocol version 13 must keep accepting the width change, got {:?}",
                result.errors
            );
        }
    }
}
