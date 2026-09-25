//! Helpers shared by every generation of `DocumentTypeRef::validate_update`
//! (`v0`, `v1`, …). Only the parts of the update-validation flow that differ
//! between generations live in the per-version modules; the config, byte-array
//! encoding and JSON-schema compatibility checks below use options selected by
//! the versioned validator.

use crate::consensus::basic::data_contract::IncompatibleDocumentTypeSchemaError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::property::{ByteArrayPropertySizes, DocumentPropertyType};
use crate::data_contract::document_type::schema::validate_schema_compatibility;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Per-update exceptions selected by the versioned validator. Defaults retain
/// the original immutable-config behavior for earlier protocol versions.
#[derive(Default)]
pub(super) struct UpdateValidationOptions {
    /// Allow the verified true-to-false deletion flag correction while both
    /// the old and new document types keep history.
    pub(super) allow_history_delete_repair: bool,
}

impl DocumentTypeRef<'_> {
    /// A byte array property whose `minItems == maxItems` is serialized as raw,
    /// fixed-length bytes with no length prefix; any other size bounds make it
    /// serialized with a variable-length (varint) length prefix. Crossing that
    /// boundary -- or changing the fixed length itself -- silently changes the
    /// on-disk layout of every already-stored document, so re-decoding old bytes
    /// against the new type misreads them. JSON-schema compatibility treats
    /// widening/removing `maxItems` as compatible, so this layout invariant must
    /// be enforced separately. Runs before `validate_schema` so it cannot be
    /// bypassed by a JSON-schema-compatible widening.
    pub(super) fn validate_byte_array_encoding_stability(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        // Mirror the encoder/decoder exactly (see `encode_value_ref_with_size`):
        // the raw, no-length-prefix path is used ONLY when BOTH bounds are present
        // and equal. Any other shape -- including an omitted `minItems` (`None`) --
        // is varint length-prefixed, so an implicit `minItems: 0` must NOT be
        // treated as fixed-length here or this guard would diverge from the actual
        // on-disk layout. `Some(n)` => fixed raw encoding of length `n`; `None` =>
        // variable (varint length-prefixed) encoding.
        fn fixed_length(sizes: &ByteArrayPropertySizes) -> Option<u16> {
            match (sizes.min_size, sizes.max_size) {
                (Some(min), Some(max)) if min == max => Some(min),
                _ => None,
            }
        }

        let new_properties = new_document_type.flattened_properties();

        for (path, old_property) in self.flattened_properties() {
            let DocumentPropertyType::ByteArray(old_sizes) = &old_property.property_type else {
                continue;
            };

            let Some(new_property) = new_properties.get(path) else {
                continue;
            };

            let DocumentPropertyType::ByteArray(new_sizes) = &new_property.property_type else {
                continue;
            };

            if fixed_length(old_sizes) != fixed_length(new_sizes) {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.data_contract_id(),
                        self.name(),
                        format!(
                            "document type can not change the byte array encoding of property \
                             '{}': changing its size bounds from (minItems: {:?}, maxItems: {:?}) \
                             to (minItems: {:?}, maxItems: {:?}) alters the on-disk layout of \
                             existing documents",
                            path,
                            old_sizes.min_size,
                            old_sizes.max_size,
                            new_sizes.min_size,
                            new_sizes.max_size,
                        ),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }

    pub(super) fn validate_config(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        self.validate_config_with_options(new_document_type, &UpdateValidationOptions::default())
    }

    /// Only protocol 14's update validator permits repairing the unusable delete
    /// flag. Earlier generations keep the original immutable-config behavior.
    pub(super) fn validate_config_with_options(
        &self,
        new_document_type: DocumentTypeRef,
        options: &UpdateValidationOptions,
    ) -> SimpleConsensusValidationResult {
        if new_document_type.creation_restriction_mode() != self.creation_restriction_mode() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change creation restriction mode: changing from {} to {}",
                        self.creation_restriction_mode(),
                        new_document_type.creation_restriction_mode()
                    ),
                )
                .into(),
            );
        }

        if new_document_type.trade_mode() != self.trade_mode() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change trade mode: changing from {} to {}",
                        self.trade_mode(),
                        new_document_type.trade_mode()
                    ),
                )
                .into(),
            );
        }

        if new_document_type.documents_transferable() != self.documents_transferable() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether its documents are transferable: changing from {} to {}",
                        self.documents_transferable(),
                        new_document_type.documents_transferable()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.documents_can_be_deleted() != self.documents_can_be_deleted()
            && !options.allow_history_delete_repair
        {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether its documents can be deleted: changing from {} to {}",
                        self.documents_can_be_deleted(),
                        new_document_type.documents_can_be_deleted()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.documents_keep_history() != self.documents_keep_history() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether it keeps history: changing from {} to {}",
                        self.documents_keep_history(),
                        new_document_type.documents_keep_history()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.documents_keep_transfer_history()
            != self.documents_keep_transfer_history()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether it keeps transfer history: changing from {} to {}",
                        self.documents_keep_transfer_history(),
                        new_document_type.documents_keep_transfer_history()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.documents_keep_purchase_history()
            != self.documents_keep_purchase_history()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether it keeps purchase history: changing from {} to {}",
                        self.documents_keep_purchase_history(),
                        new_document_type.documents_keep_purchase_history()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.documents_keep_pricing_history()
            != self.documents_keep_pricing_history()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether it keeps pricing history: changing from {} to {}",
                        self.documents_keep_pricing_history(),
                        new_document_type.documents_keep_pricing_history()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.documents_mutable() != self.documents_mutable() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether its documents are mutable: changing from {} to {}",
                        self.documents_mutable(),
                        new_document_type.documents_mutable()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.requires_identity_encryption_bounded_key()
            != self.requires_identity_encryption_bounded_key()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether it required an identity encryption bounded key: changing from {:?} to {:?}",
                        self.requires_identity_encryption_bounded_key(),
                        new_document_type.requires_identity_encryption_bounded_key()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.requires_identity_decryption_bounded_key()
            != self.requires_identity_decryption_bounded_key()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether it required an identity decryption bounded key: changing from {:?} to {:?}",
                        self.requires_identity_decryption_bounded_key(),
                        new_document_type.requires_identity_decryption_bounded_key()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.security_level_requirement() != self.security_level_requirement() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change the security level requirement for its updates: changing from {:?} to {:?}",
                        self.security_level_requirement(),
                        new_document_type.security_level_requirement()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.documents_countable() != self.documents_countable() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether its documents are countable: changing from {} to {}",
                        self.documents_countable(),
                        new_document_type.documents_countable()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.range_countable() != self.range_countable() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether it is range countable: changing from {} to {}",
                        self.range_countable(),
                        new_document_type.range_countable()
                    ),
                )
                    .into(),
            );
        }

        // Sum-tree immutability — parallels the count flags above.
        // Two checks: (1) whether the doctype is summable at all (the
        // presence/absence of `documents_summable`), and (2) the *name* of
        // the summed property. Changing either invalidates every on-disk
        // sum contribution because grovedb's sum trees aggregate `i64`
        // per merk node — a renamed property would silently double-count
        // or under-count depending on which document field gets read.
        if new_document_type.documents_summable() != self.documents_summable() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether or how its documents are summable: changing from {:?} to {:?}",
                        self.documents_summable(),
                        new_document_type.documents_summable()
                    ),
                )
                    .into(),
            );
        }

        if new_document_type.range_summable() != self.range_summable() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether it is range summable: changing from {} to {}",
                        self.range_summable(),
                        new_document_type.range_summable()
                    ),
                )
                    .into(),
            );
        }

        // indexOnly immutability: the flag selects the entire storage layout
        // (no primary-key tree, index terminals are Items keyed by the
        // terminal property). Flipping it in either direction would strand
        // every existing entry: rows with no primary storage to dereference,
        // or references whose primary rows were never written. The per-index
        // `terminal` needs no separate check here — it is a field of `Index`,
        // so `validate_index_definitions_unchanged`'s full equality
        // comparison already rejects any change to it.
        if new_document_type.index_only() != self.index_only() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether it is indexOnly: changing from {} to {}",
                        self.index_only(),
                        new_document_type.index_only()
                    ),
                )
                    .into(),
            );
        }

        // The entry payload is part of every stored entry's value layout:
        // adding, dropping or renaming a payload property would leave the
        // existing entries undecodable (and their commitments unrecomputable).
        if new_document_type.entry_payload() != self.entry_payload() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    "document type can not change its entryPayload: the listed properties are \
                     the value layout of every stored entry"
                        .to_string(),
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }

    pub(super) fn validate_schema(
        &self,
        new_document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        self.validate_schema_with_options(
            new_document_type,
            platform_version,
            &UpdateValidationOptions::default(),
        )
    }

    pub(super) fn validate_schema_with_options(
        &self,
        new_document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
        options: &UpdateValidationOptions,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // All good if schema is the same
        if self.schema() == new_document_type.schema() {
            return Ok(SimpleConsensusValidationResult::new());
        }

        let mut old_document_schema_json = match self.schema().try_to_validating_json() {
            Ok(json_value) => json_value,
            Err(e) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DataContractError::ValueDecodingError(format!(
                        "invalid existing json schema structure for document type {}: {e}",
                        self.name()
                    ))
                    .into(),
                ));
            }
        };

        let mut new_document_schema_json = match new_document_type.schema().try_to_validating_json()
        {
            Ok(json_value) => json_value,
            Err(e) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DataContractError::ValueDecodingError(format!(
                        "invalid new json schema structure for document type {}: {e}",
                        self.name()
                    ))
                    .into(),
                ));
            }
        };

        if options.allow_history_delete_repair {
            // The parsed flags already proved this is the one permitted config
            // correction. It changes neither property encoding nor history.
            // Strip only the top-level flag, including the legacy omitted-key
            // case; a property named canBeDeleted must still be validated.
            for schema in [&mut old_document_schema_json, &mut new_document_schema_json] {
                if let Some(map) = schema.as_object_mut() {
                    map.remove("canBeDeleted");
                }
            }
        }

        let compatibility_validation_result = validate_schema_compatibility(
            &old_document_schema_json,
            &new_document_schema_json,
            platform_version,
        )?;

        // Convert the compatibility errors to consensus errors
        let errors = compatibility_validation_result
            .errors
            .into_iter()
            .map(|operation| {
                IncompatibleDocumentTypeSchemaError::new(
                    self.name().clone(),
                    operation.name,
                    operation.path,
                )
                .into()
            })
            .collect();

        Ok(SimpleConsensusValidationResult::new_with_errors(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::DocumentType;
    use assert_matches::assert_matches;
    use platform_value::platform_value;
    use platform_value::Identifier;

    mod validate_config {
        use super::*;
        use std::collections::BTreeMap;

        #[test]
        fn should_return_invalid_result_when_creation_restriction_mode_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "creationRestrictionMode": 1,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "creationRestrictionMode": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change creation restriction mode: changing from Owner Only to No Restrictions"
            );
        }

        #[test]
        fn should_return_invalid_result_when_trade_mode_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "tradeMode": 1,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "tradeMode": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change trade mode: changing from Direct Purchase to No Trading"
            );
        }

        #[test]
        fn should_return_invalid_result_when_documents_transferable_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "transferable": 1,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "transferable": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether its documents are transferable: changing from Always to Never"
            );
        }

        #[test]
        fn should_return_invalid_result_when_documents_can_be_deleted_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "canBeDeleted": true,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "canBeDeleted": false,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether its documents can be deleted: changing from true to false"
            );
        }

        #[test]
        fn should_return_invalid_result_when_documents_keep_history_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "documentsKeepHistory": true,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "documentsKeepHistory": false,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether it keeps history: changing from true to false"
            );
        }

        #[test]
        fn should_return_invalid_result_when_documents_mutable_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "documentsMutable": true,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "documentsMutable": false,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether its documents are mutable: changing from true to false"
            );
        }

        #[test]
        fn should_return_invalid_result_when_requires_identity_encryption_bounded_key_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "requiresIdentityEncryptionBoundedKey": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "requiresIdentityEncryptionBoundedKey": 1,
                "additionalProperties": false,
            });

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether it required an identity encryption bounded key: changing from Some(Unique) to Some(Multiple)"
            );
        }

        #[test]
        fn should_return_invalid_result_when_requires_identity_decryption_bounded_key_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "requiresIdentityDecryptionBoundedKey": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "requiresIdentityDecryptionBoundedKey": 2,
                "additionalProperties": false,
            });

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether it required an identity decryption bounded key: changing from Some(Unique) to Some(MultipleReferenceToLatest)"
            );
        }

        #[test]
        fn should_return_invalid_result_when_security_level_requirement_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "signatureSecurityLevelRequirement": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "signatureSecurityLevelRequirement": 1,
                "additionalProperties": false,
            });

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change the security level requirement for its updates: changing from MASTER to CRITICAL"
            );
        }

        #[test]
        fn should_return_invalid_result_when_documents_countable_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "documentsCountable": true,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "documentsCountable": false,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether its documents are countable: changing from true to false"
            );
        }

        #[test]
        fn should_return_invalid_result_when_index_only_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "like";

            // A minimal valid indexOnly type: one refersTo-typed property,
            // one index over it, terminal defaulting to $ownerId.
            let index_only_schema = platform_value!({
                "type": "object",
                "indexOnly": true,
                "documentsMutable": false,
                "properties": {
                    "postId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "refersTo": { "type": "identity" },
                        "position": 0,
                    }
                },
                "required": ["postId"],
                "indices": [
                    {
                        "name": "byPost",
                        "properties": [{ "postId": "asc" }],
                    }
                ],
                "additionalProperties": false,
            });

            // The same type without indexOnly (documentsMutable kept equal
            // so the earlier mutability check doesn't fire first).
            let plain_schema = platform_value!({
                "type": "object",
                "documentsMutable": false,
                "properties": {
                    "postId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "refersTo": { "type": "identity" },
                        "position": 0,
                    }
                },
                "required": ["postId"],
                "indices": [
                    {
                        "name": "byPost",
                        "properties": [{ "postId": "asc" }],
                    }
                ],
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let make_document_type = |schema: platform_value::Value| {
                DocumentType::try_from_schema(
                    data_contract_id,
                    1,
                    config.version(),
                    document_type_name,
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

            let old_document_type = make_document_type(index_only_schema);
            let new_document_type = make_document_type(plain_schema);

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether it is indexOnly: changing from true to false"
            );
        }

        #[test]
        fn should_return_invalid_result_when_entry_payload_is_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "note";

            // Both types are valid indexOnly types over the same properties:
            // one keeps `body` in every entry's value slot, the other keys
            // by it as the terminal's last component. Every other config
            // flag is equal, so `validate_config` reaches the payload check.
            let payload_schema = platform_value!({
                "type": "object",
                "indexOnly": true,
                "documentsMutable": false,
                "properties": {
                    "postId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0,
                    },
                    "body": {
                        "type": "array",
                        "byteArray": true,
                        "maxItems": 16,
                        "position": 1,
                    }
                },
                "required": ["postId", "body"],
                "indices": [
                    {
                        "name": "byPost",
                        "properties": [{ "postId": "asc" }],
                    }
                ],
                "entryPayload": ["body"],
                "additionalProperties": false,
            });
            let keyed_schema = platform_value!({
                "type": "object",
                "indexOnly": true,
                "documentsMutable": false,
                "properties": {
                    "postId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0,
                    },
                    "body": {
                        "type": "array",
                        "byteArray": true,
                        "maxItems": 16,
                        "position": 1,
                    }
                },
                "required": ["postId", "body"],
                "indices": [
                    {
                        "name": "byPost",
                        "properties": [{ "postId": "asc" }],
                        "terminal": ["$ownerId", "body"],
                    }
                ],
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let make_document_type = |schema: platform_value::Value| {
                DocumentType::try_from_schema(
                    data_contract_id,
                    1,
                    config.version(),
                    document_type_name,
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

            let old_document_type = make_document_type(payload_schema);
            let new_document_type = make_document_type(keyed_schema);

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message().starts_with("document type can not change its entryPayload")
            );

            // The unchanged pair passes the same check.
            let result = old_document_type
                .as_ref()
                .validate_config(old_document_type.as_ref());
            assert!(result.errors.is_empty(), "{:?}", result.errors);
        }

        #[test]
        fn should_return_invalid_result_when_range_countable_is_changed() {
            // documents_countable must remain equal across old/new so that
            // validate_config reaches the range_countable check below it.
            // Setting documentsCountable: true on both keeps the
            // documents_countable() getter true regardless of range_countable.
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "documentsCountable": true,
                "rangeCountable": false,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "documentsCountable": true,
                "rangeCountable": true,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether it is range countable: changing from false to true"
            );
        }

        /// Builds old/new document types from two schemas and runs
        /// `validate_config`, asserting the exact rejection message. The
        /// per-flag tests below only differ in one schema keyword, so the
        /// boilerplate lives here.
        fn assert_config_change_rejected(
            old_schema: platform_value::Value,
            new_schema: platform_value::Value,
            expected_message: &str,
        ) {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                old_schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                new_schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_config(new_document_type.as_ref());

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == expected_message
            );
        }

        fn schema_with_keep_flag(flag: &str, value: bool) -> platform_value::Value {
            platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                flag: value,
                "additionalProperties": false,
            })
        }

        #[test]
        fn should_return_invalid_result_when_documents_keep_transfer_history_is_changed() {
            assert_config_change_rejected(
                schema_with_keep_flag("keepsTransferHistory", true),
                schema_with_keep_flag("keepsTransferHistory", false),
                "document type can not change whether it keeps transfer history: changing from true to false",
            );
        }

        #[test]
        fn should_return_invalid_result_when_documents_keep_purchase_history_is_changed() {
            assert_config_change_rejected(
                schema_with_keep_flag("keepsPurchaseHistory", true),
                schema_with_keep_flag("keepsPurchaseHistory", false),
                "document type can not change whether it keeps purchase history: changing from true to false",
            );
        }

        #[test]
        fn should_return_invalid_result_when_documents_keep_pricing_history_is_changed() {
            assert_config_change_rejected(
                schema_with_keep_flag("keepsPricingHistory", true),
                schema_with_keep_flag("keepsPricingHistory", false),
                "document type can not change whether it keeps pricing history: changing from true to false",
            );
        }

        /// `documentsSummable` must name an integer property listed in
        /// `required`, so the summable schemas carry an `amount` field.
        fn schema_with_summable(
            documents_summable: bool,
            range_summable: bool,
        ) -> platform_value::Value {
            let mut schema = platform_value!({
                "type": "object",
                "properties": {
                    "amount": {
                        "type": "integer",
                        "position": 0,
                    }
                },
                "required": ["amount"],
                "additionalProperties": false,
            });
            let map = schema.as_map_mut().expect("schema must be a map");
            if documents_summable {
                map.push(("documentsSummable".into(), "amount".into()));
            }
            if range_summable {
                map.push(("rangeSummable".into(), true.into()));
            }
            schema
        }

        #[test]
        fn should_return_invalid_result_when_documents_summable_is_changed() {
            assert_config_change_rejected(
                schema_with_summable(true, false),
                schema_with_summable(false, false),
                "document type can not change whether or how its documents are summable: changing from Some(\"amount\") to None",
            );
        }

        #[test]
        fn should_return_invalid_result_when_range_summable_is_changed() {
            // `documentsSummable` stays equal across old and new so that
            // validate_config reaches the range_summable check below it
            // (mirrors the range_countable test above).
            assert_config_change_rejected(
                schema_with_summable(true, false),
                schema_with_summable(true, true),
                "document type can not change whether it is range summable: changing from false to true",
            );
        }
    }

    mod validate_schema {
        use super::*;
        use crate::consensus::basic::BasicError;
        use crate::data_contract::document_type::accessors::DocumentTypeV0MutGetters;
        use std::collections::BTreeMap;

        #[test]
        fn should_pass_when_schema_is_not_changed() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "signatureSecurityLevelRequirement": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema.clone(),
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_schema(new_document_type.as_ref(), platform_version)
                .expect("failed to validate schema compatibility");

            assert!(result.is_valid());
        }

        #[test]
        fn should_return_invalid_result_when_schemas_are_not_backward_compatible() {
            let platform_version = PlatformVersion::latest();
            let data_contract_id = Identifier::random();
            let document_type_name = "test";

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "signatureSecurityLevelRequirement": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema.clone(),
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test": {
                        "type": "number",
                        "position": 0,
                    }
                },
                "signatureSecurityLevelRequirement": 0,
                "additionalProperties": false,
            });

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                document_type_name,
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create new document type");

            let result = old_document_type
                .as_ref()
                .validate_schema(new_document_type.as_ref(), platform_version)
                .expect("failed to validate schema compatibility");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IncompatibleDocumentTypeSchemaError(e)
                )] if e.operation() == "replace" && e.property_path() == "/properties/test/type"
            );
        }

        fn identifier_document_type(
            refers_to: Option<platform_value::Value>,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let mut to_user_id = platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 0
            });

            if let Some(refers_to) = refers_to {
                to_user_id
                    .insert("refersTo".to_string(), refers_to)
                    .expect("should insert refersTo");
            }

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "toUserId": to_user_id
                },
                "signatureSecurityLevelRequirement": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            DocumentType::try_from_schema(
                Identifier::random(),
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

        #[test]
        fn should_return_invalid_result_when_refers_to_is_added() {
            let platform_version = PlatformVersion::latest();

            let old_document_type = identifier_document_type(None, platform_version);
            let new_document_type = identifier_document_type(
                Some(platform_value!({ "type": "identity" })),
                platform_version,
            );

            let result = old_document_type
                .as_ref()
                .validate_schema(new_document_type.as_ref(), platform_version)
                .expect("failed to validate schema compatibility");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IncompatibleDocumentTypeSchemaError(e)
                )] if e.operation() == "add"
                    && e.property_path() == "/properties/toUserId/refersTo"
            );
        }

        #[test]
        fn should_return_invalid_result_when_refers_to_is_modified() {
            let platform_version = PlatformVersion::latest();

            let old_document_type = identifier_document_type(
                Some(platform_value!({ "type": "identity" })),
                platform_version,
            );
            let new_document_type = identifier_document_type(
                Some(platform_value!({ "type": "contract" })),
                platform_version,
            );

            let result = old_document_type
                .as_ref()
                .validate_schema(new_document_type.as_ref(), platform_version)
                .expect("failed to validate schema compatibility");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IncompatibleDocumentTypeSchemaError(e)
                )] if e.operation() == "replace"
                    && e.property_path() == "/properties/toUserId/refersTo/type"
            );
        }

        /// A property's document reference cannot trade the permanence
        /// promise existing documents were written under for the weaker
        /// one, nor the other way round (which would brick every write
        /// against a deletable target type).
        #[test]
        fn should_return_invalid_result_when_a_document_reference_changes_kind() {
            let platform_version = PlatformVersion::latest();

            for (old_kind, new_kind) in [
                ("permanentDocument", "deletableDocument"),
                ("deletableDocument", "permanentDocument"),
            ] {
                let old_document_type = identifier_document_type(
                    Some(platform_value!({ "type": old_kind, "documentType": "note" })),
                    platform_version,
                );
                let new_document_type = identifier_document_type(
                    Some(platform_value!({ "type": new_kind, "documentType": "note" })),
                    platform_version,
                );

                let result = old_document_type
                    .as_ref()
                    .validate_schema(new_document_type.as_ref(), platform_version)
                    .expect("failed to validate schema compatibility");

                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IncompatibleDocumentTypeSchemaError(e)
                    )] if e.operation() == "replace"
                        && e.property_path() == "/properties/toUserId/refersTo/type"
                );
            }
        }
        #[test]
        fn should_return_invalid_result_when_a_contract_reference_requirement_changes() {
            let platform_version = PlatformVersion::latest();

            for (old_fields, new_fields, changed_path) in [
                (
                    platform_value!({ "type": "contract" }),
                    platform_value!({ "type": "contract", "contractRequirements": { "moderation": "elected" } }),
                    "/properties/toUserId/refersTo/contractRequirements",
                ),
                (
                    platform_value!({ "type": "contract", "contractRequirements": { "moderation": "elected" } }),
                    platform_value!({ "type": "contract" }),
                    "/properties/toUserId/refersTo/contractRequirements",
                ),
                (
                    platform_value!({ "type": "contract", "contractRequirements": { "minimumAgeSeconds": 3600 } }),
                    platform_value!({ "type": "contract", "contractRequirements": { "minimumAgeSeconds": 7200 } }),
                    "/properties/toUserId/refersTo/contractRequirements/minimumAgeSeconds",
                ),
                (
                    platform_value!({ "type": "contract", "contractRequirements": { "minimumSecondsSinceUpdate": 60 } }),
                    platform_value!({ "type": "contract", "contractRequirements": { "minimumSecondsSinceUpdate": 61 } }),
                    "/properties/toUserId/refersTo/contractRequirements/minimumSecondsSinceUpdate",
                ),
                (
                    platform_value!({ "type": "contract", "contractRequirements": { "owner": "self" } }),
                    platform_value!({ "type": "contract", "contractRequirements": { "owner": "other" } }),
                    "/properties/toUserId/refersTo/contractRequirements/owner",
                ),
                (
                    platform_value!({ "type": "contract", "contractRequirements": { "ownerProtected": true } }),
                    platform_value!({ "type": "contract", "contractRequirements": { "ownerProtected": false } }),
                    "/properties/toUserId/refersTo/contractRequirements/ownerProtected",
                ),
                (
                    platform_value!({ "type": "contract", "contractRequirements": { "owner": "self" } }),
                    platform_value!({ "type": "contract", "contractRequirements": { "owner": "self", "readonly": true } }),
                    "/properties/toUserId/refersTo/contractRequirements/readonly",
                ),
                (
                    platform_value!({ "type": "contract", "contractRequirements": { "owner": "self", "keepsHistory": true } }),
                    platform_value!({ "type": "contract", "contractRequirements": { "owner": "self" } }),
                    "/properties/toUserId/refersTo/contractRequirements/keepsHistory",
                ),
            ] {
                let old_document_type =
                    identifier_document_type(Some(old_fields), platform_version);
                let new_document_type =
                    identifier_document_type(Some(new_fields), platform_version);

                let result = old_document_type
                    .as_ref()
                    .validate_schema(new_document_type.as_ref(), platform_version)
                    .expect("failed to validate schema compatibility");

                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IncompatibleDocumentTypeSchemaError(e)
                    )] if e.property_path() == changed_path
                );
            }
        }

        #[test]
        fn should_return_invalid_result_when_an_identity_key_reference_requirement_changes() {
            let platform_version = PlatformVersion::latest();

            for (old_fields, new_fields, changed_path) in [
                (
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex" }),
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex", "keyRequirements": { "purpose": "decryption" } }),
                    "/properties/toUserId/refersTo/keyRequirements",
                ),
                (
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex", "keyRequirements": { "purpose": "decryption" } }),
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex" }),
                    "/properties/toUserId/refersTo/keyRequirements",
                ),
                (
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex", "keyRequirements": { "purpose": "decryption" } }),
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex", "keyRequirements": { "purpose": "encryption" } }),
                    "/properties/toUserId/refersTo/keyRequirements/purpose",
                ),
                (
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex", "keyRequirements": { "boundTo": "test" } }),
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex", "keyRequirements": { "boundTo": "other" } }),
                    "/properties/toUserId/refersTo/keyRequirements/boundTo",
                ),
                (
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex", "keyRequirements": { "purpose": "decryption" } }),
                    platform_value!({ "type": "identityPublicKey", "keyIdProperty": "toKeyIndex", "keyRequirements": { "purpose": "decryption", "boundTo": "test" } }),
                    "/properties/toUserId/refersTo/keyRequirements/boundTo",
                ),
            ] {
                let old_document_type =
                    identifier_document_type(Some(old_fields), platform_version);
                let new_document_type =
                    identifier_document_type(Some(new_fields), platform_version);

                let result = old_document_type
                    .as_ref()
                    .validate_schema(new_document_type.as_ref(), platform_version)
                    .expect("failed to validate schema compatibility");

                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IncompatibleDocumentTypeSchemaError(e)
                    )] if e.property_path() == changed_path
                );
            }
        }

        /// `reasons`, a typed array of identifiers, with `refersTo` on its items as given,
        /// next to a `topic` an agreement can name.
        fn element_reference_document_type(
            refers_to: Option<platform_value::Value>,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let mut items = platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier"
            });
            if let Some(refers_to) = refers_to {
                items
                    .insert("refersTo".to_string(), refers_to)
                    .expect("should insert refersTo");
            }

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "reasons": {
                        "type": "array",
                        "maxItems": 64,
                        "items": items,
                        "position": 0
                    },
                    "topic": { "type": "string", "maxLength": 32, "position": 1 }
                },
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            DocumentType::try_from_schema(
                Identifier::random(),
                1,
                config.version(),
                "submittedCharter",
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

        /// An element reference is frozen like a scalar one: stored documents were
        /// checked against the declaration they were written under, so adding,
        /// removing or changing it is an incompatible schema change.
        #[test]
        fn should_refuse_a_contract_update_that_changes_an_element_refers_to() {
            let platform_version = PlatformVersion::latest();
            let permanent =
                platform_value!({ "type": "permanentDocument", "documentType": "reason" });

            for (old_refers_to, new_refers_to, changed_path) in [
                (
                    None,
                    Some(permanent.clone()),
                    "/properties/reasons/items/refersTo",
                ),
                (
                    Some(permanent.clone()),
                    None,
                    "/properties/reasons/items/refersTo",
                ),
                (
                    Some(permanent.clone()),
                    Some(
                        platform_value!({ "type": "deletableDocument", "documentType": "reason" }),
                    ),
                    "/properties/reasons/items/refersTo/type",
                ),
                (
                    Some(permanent.clone()),
                    Some(platform_value!({ "type": "permanentDocument", "documentType": "rule" })),
                    "/properties/reasons/items/refersTo/documentType",
                ),
                (
                    Some(permanent.clone()),
                    Some(platform_value!({
                        "type": "permanentDocument",
                        "documentType": "reason",
                        "propertyAgreement": { "topic": "topic" }
                    })),
                    "/properties/reasons/items/refersTo/propertyAgreement",
                ),
            ] {
                let old_document_type =
                    element_reference_document_type(old_refers_to.clone(), platform_version);
                let new_document_type =
                    element_reference_document_type(new_refers_to.clone(), platform_version);

                let result = old_document_type
                    .as_ref()
                    .validate_update(new_document_type.as_ref(), 2, platform_version)
                    .expect("validate_update should not error");

                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IncompatibleDocumentTypeSchemaError(e)
                    )] if e.property_path() == changed_path,
                    "{old_refers_to:?} -> {new_refers_to:?}: {:?}",
                    result.errors
                );
            }

            // An unchanged declaration is no change
            let document_type = element_reference_document_type(Some(permanent), platform_version);
            let result = document_type
                .as_ref()
                .validate_update(document_type.as_ref(), 2, platform_version)
                .expect("validate_update should not error");
            assert!(result.is_valid(), "{:?}", result.errors);
        }

        #[test]
        fn should_return_invalid_result_when_a_document_reference_lookup_changes() {
            let platform_version = PlatformVersion::latest();
            let owner_lookup = |index: &str| {
                platform_value!({
                    "type": "permanentDocument",
                    "documentType": "note",
                    "lookup": { "index": index, "keys": { "$ownerId": "." } }
                })
            };

            for (old_refers_to, new_refers_to, changed_path) in [
                (
                    platform_value!({ "type": "permanentDocument", "documentType": "note" }),
                    owner_lookup("byOwner"),
                    "/properties/toUserId/refersTo/lookup",
                ),
                (
                    owner_lookup("byOwner"),
                    platform_value!({ "type": "permanentDocument", "documentType": "note" }),
                    "/properties/toUserId/refersTo/lookup",
                ),
                (
                    owner_lookup("byOwner"),
                    owner_lookup("byAuthor"),
                    "/properties/toUserId/refersTo/lookup/index",
                ),
            ] {
                let old_document_type =
                    identifier_document_type(Some(old_refers_to), platform_version);
                let new_document_type =
                    identifier_document_type(Some(new_refers_to), platform_version);

                let result = old_document_type
                    .as_ref()
                    .validate_schema(new_document_type.as_ref(), platform_version)
                    .expect("failed to validate schema compatibility");

                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IncompatibleDocumentTypeSchemaError(e)
                    )] if e.property_path() == changed_path
                );
            }
        }

        /// A reference expression is frozen like a single target: documents were checked
        /// against the expression they were written under, so turning a target into an
        /// expression or back, adding, removing or reordering an operand (the order decides
        /// which error a writer sees), swapping `anyOf` for `allOf`, nesting deeper, or
        /// changing a leaf is an incompatible schema change. `anyOf` and `allOf` inside
        /// `refersTo` are the declaration's data, never read as the JSON Schema keywords.
        #[test]
        fn should_return_invalid_result_when_a_reference_expression_changes() {
            let platform_version = PlatformVersion::latest();
            let identity = platform_value!({ "type": "identity" });
            let note = platform_value!({ "type": "permanentDocument", "documentType": "note" });
            let memo = platform_value!({ "type": "permanentDocument", "documentType": "memo" });
            let any_of = |targets: Vec<platform_value::Value>| platform_value!({ "anyOf": platform_value::Value::Array(targets) });
            let all_of = |targets: Vec<platform_value::Value>| platform_value!({ "allOf": platform_value::Value::Array(targets) });

            for (old_refers_to, new_refers_to) in [
                (
                    identity.clone(),
                    any_of(vec![identity.clone(), note.clone()]),
                ),
                (any_of(vec![identity.clone(), note.clone()]), note.clone()),
                (
                    any_of(vec![identity.clone(), note.clone()]),
                    any_of(vec![identity.clone(), note.clone(), memo.clone()]),
                ),
                (
                    any_of(vec![identity.clone(), note.clone(), memo.clone()]),
                    any_of(vec![identity.clone(), note.clone()]),
                ),
                (
                    any_of(vec![identity.clone(), note.clone()]),
                    any_of(vec![note.clone(), identity.clone()]),
                ),
                (
                    any_of(vec![identity.clone(), note.clone()]),
                    any_of(vec![identity.clone(), memo.clone()]),
                ),
                (
                    any_of(vec![identity.clone(), note.clone()]),
                    all_of(vec![identity.clone(), note.clone()]),
                ),
                (
                    any_of(vec![identity.clone(), note.clone()]),
                    any_of(vec![
                        identity.clone(),
                        all_of(vec![note.clone(), memo.clone()]),
                    ]),
                ),
                (
                    all_of(vec![
                        identity.clone(),
                        any_of(vec![note.clone(), memo.clone()]),
                    ]),
                    all_of(vec![
                        identity.clone(),
                        any_of(vec![memo.clone(), note.clone()]),
                    ]),
                ),
            ] {
                let old_document_type =
                    identifier_document_type(Some(old_refers_to.clone()), platform_version);
                let new_document_type =
                    identifier_document_type(Some(new_refers_to.clone()), platform_version);

                let result = old_document_type
                    .as_ref()
                    .validate_schema(new_document_type.as_ref(), platform_version)
                    .expect("failed to validate schema compatibility");

                assert!(
                    !result.errors.is_empty(),
                    "{old_refers_to:?} -> {new_refers_to:?} should be incompatible"
                );
                for error in &result.errors {
                    assert_matches!(
                        error,
                        ConsensusError::BasicError(
                            BasicError::IncompatibleDocumentTypeSchemaError(e)
                        ) if e.property_path().starts_with("/properties/toUserId/refersTo"),
                        "{old_refers_to:?} -> {new_refers_to:?}"
                    );
                }
            }

            // An unchanged expression is no change
            let document_type = identifier_document_type(
                Some(any_of(vec![identity, all_of(vec![note, memo])])),
                platform_version,
            );
            let result = document_type
                .as_ref()
                .validate_schema(document_type.as_ref(), platform_version)
                .expect("failed to validate schema compatibility");
            assert!(result.is_valid(), "{:?}", result.errors);
        }

        /// The same holds on the elements of a typed array.
        #[test]
        fn should_refuse_a_contract_update_that_changes_an_element_reference_expression() {
            let platform_version = PlatformVersion::latest();
            let reason = platform_value!({ "type": "permanentDocument", "documentType": "reason" });
            let any_of = platform_value!({
                "anyOf": [{ "type": "identity" }, { "type": "permanentDocument", "documentType": "reason" }]
            });

            let all_of = platform_value!({
                "allOf": [{ "type": "identity" }, { "type": "permanentDocument", "documentType": "reason" }]
            });
            for (old_refers_to, new_refers_to) in [
                (reason.clone(), any_of.clone()),
                (any_of.clone(), reason.clone()),
                (any_of.clone(), all_of.clone()),
            ] {
                let old_document_type =
                    element_reference_document_type(Some(old_refers_to), platform_version);
                let new_document_type =
                    element_reference_document_type(Some(new_refers_to), platform_version);

                let result = old_document_type
                    .as_ref()
                    .validate_update(new_document_type.as_ref(), 2, platform_version)
                    .expect("validate_update should not error");

                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IncompatibleDocumentTypeSchemaError(e)
                    ), ..] if e.property_path().starts_with("/properties/reasons/items/refersTo"),
                    "{:?}",
                    result.errors
                );
            }
        }

        /// `reasons`, a typed array of identifiers, with `refersTo` on its items as given,
        /// parsed as a contract read back from state is, so a `listElement` declaration
        /// needs no `$id` property to exist beside it.
        fn element_list_document_type(
            refers_to: Option<platform_value::Value>,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let mut items = platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier"
            });
            if let Some(refers_to) = refers_to {
                items
                    .insert("refersTo".to_string(), refers_to)
                    .expect("should insert refersTo");
            }
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "reasons": { "type": "array", "maxItems": 8, "items": items, "position": 0 }
                },
                "additionalProperties": false,
            });
            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");
            DocumentType::try_from_schema(
                Identifier::random(),
                1,
                config.version(),
                "resignation",
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

        /// A list element reference is frozen like every other declaration: stored
        /// documents were checked against the list it names, on the document its `$id`
        /// pair names, so adding, removing or changing it (on an identifier property or on
        /// the elements of a typed array) is an incompatible schema change.
        #[test]
        fn should_return_invalid_result_when_a_list_element_reference_changes() {
            let platform_version = PlatformVersion::latest();
            let list_element = |id_property: &str, in_list: &str| {
                platform_value!({
                    "type": "listElement",
                    "documentType": "electedCharter",
                    "propertyAgreement": { id_property: "$id" },
                    "inList": in_list
                })
            };
            let members = list_element("electedCharterId", "members");

            for (old_refers_to, new_refers_to, changed_path) in [
                (None, Some(members.clone()), "/refersTo"),
                (Some(members.clone()), None, "/refersTo"),
                (
                    Some(platform_value!({
                        "type": "permanentDocument",
                        "documentType": "electedCharter"
                    })),
                    Some(members.clone()),
                    "/refersTo/type",
                ),
                (
                    Some(members.clone()),
                    Some(list_element("electedCharterId", "seats")),
                    "/refersTo/inList",
                ),
                (
                    Some(members.clone()),
                    Some(list_element("otherCharterId", "members")),
                    "/refersTo/propertyAgreement",
                ),
            ] {
                for (old_document_type, new_document_type, property_path) in [
                    (
                        identifier_document_type(old_refers_to.clone(), platform_version),
                        identifier_document_type(new_refers_to.clone(), platform_version),
                        "/properties/toUserId",
                    ),
                    (
                        element_list_document_type(old_refers_to.clone(), platform_version),
                        element_list_document_type(new_refers_to.clone(), platform_version),
                        "/properties/reasons/items",
                    ),
                ] {
                    let result = old_document_type
                        .as_ref()
                        .validate_update(new_document_type.as_ref(), 2, platform_version)
                        .expect("validate_update should not error");

                    // Swapping the target kind also adds the keywords only a
                    // list element takes, each its own incompatible change;
                    // a renamed pair is a removal and an addition under
                    // propertyAgreement
                    let expected_path = format!("{property_path}{changed_path}");
                    let changed_paths: Vec<&str> = result
                        .errors
                        .iter()
                        .map(|error| match error {
                            ConsensusError::BasicError(
                                BasicError::IncompatibleDocumentTypeSchemaError(e),
                            ) => e.property_path(),
                            other => panic!("expected an incompatible schema change, got {other}"),
                        })
                        .collect();
                    assert!(
                        changed_paths
                            .iter()
                            .any(|changed| changed.starts_with(expected_path.as_str())),
                        "{old_refers_to:?} -> {new_refers_to:?}: {changed_paths:?}"
                    );
                }
            }

            // An unchanged declaration is no change
            let document_type = identifier_document_type(Some(members), platform_version);
            let result = document_type
                .as_ref()
                .validate_update(document_type.as_ref(), 2, platform_version)
                .expect("validate_update should not error");
            assert!(result.is_valid(), "{:?}", result.errors);
        }

        /// `toUserId` and `delegateId`, two identifier properties, with `distinctFrom` on
        /// `delegateId` as given.
        fn distinct_from_document_type(
            distinct_from: Option<&str>,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let mut delegate_id = platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 1
            });
            if let Some(distinct_from) = distinct_from {
                delegate_id
                    .insert("distinctFrom".to_string(), distinct_from.into())
                    .expect("should insert distinctFrom");
            }

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "toUserId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0
                    },
                    "delegateId": delegate_id
                },
                "signatureSecurityLevelRequirement": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            DocumentType::try_from_schema(
                Identifier::random(),
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

        #[test]
        fn should_return_invalid_result_when_distinct_from_is_added_changed_or_removed() {
            let platform_version = PlatformVersion::latest();

            for (old_distinct_from, new_distinct_from) in [
                (None, Some("$ownerId")),
                (Some("$ownerId"), Some("toUserId")),
                (Some("toUserId"), None),
            ] {
                let old_document_type =
                    distinct_from_document_type(old_distinct_from, platform_version);
                let new_document_type =
                    distinct_from_document_type(new_distinct_from, platform_version);

                let result = old_document_type
                    .as_ref()
                    .validate_schema(new_document_type.as_ref(), platform_version)
                    .expect("failed to validate schema compatibility");

                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IncompatibleDocumentTypeSchemaError(e)
                    )] if e.property_path() == "/properties/delegateId/distinctFrom",
                    "{old_distinct_from:?} -> {new_distinct_from:?}"
                );
            }
        }

        #[test]
        fn should_return_valid_result_when_distinct_from_is_unchanged() {
            let platform_version = PlatformVersion::latest();

            let old_document_type = distinct_from_document_type(Some("$ownerId"), platform_version);
            let new_document_type = distinct_from_document_type(Some("$ownerId"), platform_version);

            let result = old_document_type
                .as_ref()
                .validate_schema(new_document_type.as_ref(), platform_version)
                .expect("failed to validate schema compatibility");

            assert!(result.is_valid(), "{:?}", result.errors);
        }

        /// A `message` type whose `senderKeyId` is a `u32` key id, carrying
        /// `refers_to` when given.
        fn key_id_document_type(
            refers_to: Option<platform_value::Value>,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let mut sender_key_id = platform_value!({
                "type": "integer",
                "minimum": 0,
                "maximum": 4294967295u64,
                "position": 0
            });

            if let Some(refers_to) = refers_to {
                sender_key_id
                    .insert("refersTo".to_string(), refers_to)
                    .expect("should insert refersTo");
            }

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "senderKeyId": sender_key_id
                },
                "signatureSecurityLevelRequirement": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            DocumentType::try_from_schema(
                Identifier::random(),
                1,
                config.version(),
                "message",
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

        /// The key id form of `identityPublicKey` is a `refersTo` like any
        /// other: adding it, removing it or changing what it names is an
        /// incompatible change, so a contract can neither start nor stop
        /// checking a stored key id against the owner's keys.
        #[test]
        fn should_return_invalid_result_when_a_key_id_reference_is_added_removed_or_changed() {
            let platform_version = PlatformVersion::latest();
            let owner_key = platform_value!({
                "type": "identityPublicKey",
                "identityProperty": "$ownerId"
            });

            for (old_refers_to, new_refers_to, operation, changed_path) in [
                (
                    None,
                    Some(owner_key.clone()),
                    "add",
                    "/properties/senderKeyId/refersTo",
                ),
                (
                    Some(owner_key.clone()),
                    None,
                    "remove",
                    "/properties/senderKeyId/refersTo",
                ),
                // `identityProperty` admits one value today; what a change
                // would look like is the reference losing it for the other
                // spelling of the same reference type, which is refused
                // before the parser is even asked whether it can be parsed
                (
                    Some(owner_key.clone()),
                    Some(platform_value!({
                        "type": "identityPublicKey",
                        "identityProperty": "$creatorId"
                    })),
                    "replace",
                    "/properties/senderKeyId/refersTo/identityProperty",
                ),
            ] {
                let old_document_type = key_id_document_type(old_refers_to, platform_version);
                let new_document_type = if new_refers_to
                    .as_ref()
                    .is_some_and(|value| value.to_string().contains("creatorId"))
                {
                    // Not parseable at all: compare the schemas directly
                    // through a type built without the reference, then swap
                    // the schema in
                    let mut document_type = key_id_document_type(None, platform_version);
                    document_type
                        .schema_mut()
                        .get_mut("properties")
                        .expect("the schema should be a map")
                        .expect("properties should exist")
                        .get_mut("senderKeyId")
                        .expect("properties should be a map")
                        .expect("senderKeyId should exist")
                        .insert(
                            "refersTo".to_string(),
                            new_refers_to.clone().expect("a refersTo value"),
                        )
                        .expect("should insert refersTo");
                    document_type
                } else {
                    key_id_document_type(new_refers_to, platform_version)
                };

                let result = old_document_type
                    .as_ref()
                    .validate_schema(new_document_type.as_ref(), platform_version)
                    .expect("failed to validate schema compatibility");

                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IncompatibleDocumentTypeSchemaError(e)
                    )] if e.operation() == operation && e.property_path() == changed_path,
                    "{operation} at {changed_path}: {:?}", result.errors
                );
            }
        }
    }

    mod encrypted_for {
        use super::*;
        use crate::consensus::basic::BasicError;
        use crate::data_contract::config::DataContractConfig;
        use platform_value::platform_value;
        use std::collections::BTreeMap;

        /// A document type with a recipient, two key ids and an
        /// `encryptedMessage` carrying `encrypted_for` when given.
        fn encrypted_document_type(
            encrypted_for: Option<platform_value::Value>,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let mut encrypted_message = platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 1040,
                "position": 3
            });
            if let Some(encrypted_for) = encrypted_for {
                encrypted_message
                    .insert("encryptedFor".to_string(), encrypted_for)
                    .expect("should insert encryptedFor");
            }

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "recipientId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0
                    },
                    "recipientKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295_u64, "position": 1 },
                    "senderKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295_u64, "position": 2 },
                    "encryptedMessage": encrypted_message
                },
                "signatureSecurityLevelRequirement": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            DocumentType::try_from_schema(
                Identifier::random(),
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

        fn declaration(recipient: &str) -> platform_value::Value {
            platform_value!({
                "recipient": recipient,
                "recipientKey": "recipientKeyId",
                "senderKey": "senderKeyId",
                "scheme": "ecdh-secp256k1-aes256-cbc"
            })
        }

        /// Documents written under one recipe could not be read under another,
        /// so adding, removing or changing the declaration is incompatible.
        #[test]
        fn should_return_invalid_result_when_encrypted_for_is_added_removed_or_changed() {
            let platform_version = PlatformVersion::latest();

            for (old_declaration, new_declaration, changed_path) in [
                (
                    None,
                    Some(declaration("recipientId")),
                    "/properties/encryptedMessage/encryptedFor",
                ),
                (
                    Some(declaration("recipientId")),
                    None,
                    "/properties/encryptedMessage/encryptedFor",
                ),
                (
                    Some(declaration("recipientId")),
                    Some(declaration("$ownerId")),
                    "/properties/encryptedMessage/encryptedFor/recipient",
                ),
            ] {
                let old_document_type = encrypted_document_type(old_declaration, platform_version);
                let new_document_type = encrypted_document_type(new_declaration, platform_version);

                let result = old_document_type
                    .as_ref()
                    .validate_schema(new_document_type.as_ref(), platform_version)
                    .expect("failed to validate schema compatibility");

                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IncompatibleDocumentTypeSchemaError(e)
                    )] if e.property_path() == changed_path
                );
            }
        }

        #[test]
        fn should_return_valid_result_when_encrypted_for_is_unchanged() {
            let platform_version = PlatformVersion::latest();

            let old_document_type =
                encrypted_document_type(Some(declaration("recipientId")), platform_version);
            let new_document_type =
                encrypted_document_type(Some(declaration("recipientId")), platform_version);

            let result = old_document_type
                .as_ref()
                .validate_schema(new_document_type.as_ref(), platform_version)
                .expect("failed to validate schema compatibility");

            assert!(result.is_valid(), "{:?}", result.errors);
        }
    }

    mod max_bytes {
        use super::*;
        use crate::consensus::basic::BasicError;
        use crate::data_contract::config::DataContractConfig;
        use std::collections::BTreeMap;

        /// A string `note` with `maxBytes` as `note_bound`, and a typed string array
        /// `tags` whose `items` carry `maxBytes` as `tags_bound`.
        fn document_type(
            note_bound: Option<u64>,
            tags_bound: Option<u64>,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let mut note = platform_value!({ "type": "string", "maxLength": 64, "position": 0 });
            if let Some(max_bytes) = note_bound {
                note.insert("maxBytes".to_string(), max_bytes.into())
                    .expect("should insert maxBytes");
            }
            let mut items = platform_value!({ "type": "string", "maxLength": 16 });
            if let Some(max_bytes) = tags_bound {
                items
                    .insert("maxBytes".to_string(), max_bytes.into())
                    .expect("should insert maxBytes");
            }

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "note": note,
                    "tags": { "type": "array", "maxItems": 4, "items": items, "position": 1 }
                },
                "signatureSecurityLevelRequirement": 0,
                "additionalProperties": false,
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            DocumentType::try_from_schema(
                Identifier::random(),
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

        fn compatibility(
            old: (Option<u64>, Option<u64>),
            new: (Option<u64>, Option<u64>),
        ) -> SimpleConsensusValidationResult {
            let platform_version = PlatformVersion::latest();
            let old_document_type = document_type(old.0, old.1, platform_version);
            let new_document_type = document_type(new.0, new.1, platform_version);
            old_document_type
                .as_ref()
                .validate_schema(new_document_type.as_ref(), platform_version)
                .expect("failed to validate schema compatibility")
        }

        /// `maxBytes` moves like `maxLength`: every stored string still fits a
        /// raised or dropped bound, on a property and on typed array elements.
        #[test]
        fn should_return_valid_result_when_max_bytes_is_raised_or_removed() {
            for (old_bound, new_bound) in [(Some(8), Some(16)), (Some(8), None), (Some(8), Some(8))]
            {
                for (old, new) in [
                    ((old_bound, None), (new_bound, None)),
                    ((None, old_bound), (None, new_bound)),
                ] {
                    let result = compatibility(old, new);
                    assert!(result.is_valid(), "{old:?} -> {new:?}: {:?}", result.errors);
                }
            }
        }

        /// A stored string may be longer than a new or lowered bound.
        #[test]
        fn should_return_invalid_result_when_max_bytes_is_added_or_lowered() {
            for (old_bound, new_bound) in [(None, Some(8)), (Some(16), Some(8))] {
                for (old, new, changed_path) in [
                    (
                        (old_bound, None),
                        (new_bound, None),
                        "/properties/note/maxBytes",
                    ),
                    (
                        (None, old_bound),
                        (None, new_bound),
                        "/properties/tags/items/maxBytes",
                    ),
                ] {
                    let result = compatibility(old, new);
                    assert_matches!(
                        result.errors.as_slice(),
                        [ConsensusError::BasicError(
                            BasicError::IncompatibleDocumentTypeSchemaError(e)
                        )] if e.property_path() == changed_path,
                        "{old:?} -> {new:?}"
                    );
                }
            }
        }
    }

    mod validate_byte_array_encoding {
        use super::*;
        use std::collections::BTreeMap;

        fn document_type_with_byte_array(
            byte_array: platform_value::Value,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": { "blob": byte_array },
                "additionalProperties": false,
            });
            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");
            DocumentType::try_from_schema(
                Identifier::random(),
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

        // Exercises the PUBLIC `validate_update` dispatcher (latest protocol
        // version), so it also covers the dispatch into the current
        // generation (v1 as of protocol v14).
        fn validate_update_latest(
            old_ba: platform_value::Value,
            new_ba: platform_value::Value,
        ) -> SimpleConsensusValidationResult {
            let platform_version = PlatformVersion::latest();
            let old = document_type_with_byte_array(old_ba, platform_version);
            let new = document_type_with_byte_array(new_ba, platform_version);
            old.as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error")
        }

        fn assert_rejected(old_ba: platform_value::Value, new_ba: platform_value::Value) {
            let result = validate_update_latest(old_ba, new_ba);
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message().contains("byte array encoding")
            );
        }

        fn assert_accepted(old_ba: platform_value::Value, new_ba: platform_value::Value) {
            let result = validate_update_latest(old_ba, new_ba);
            assert!(
                result.is_valid(),
                "expected the update to be accepted, got {:?}",
                result.errors
            );
        }

        #[test]
        fn rejects_widening_fixed_byte_array_max_items() {
            // The exact attack: a fixed (raw, no length prefix) 32-byte field
            // widened to min 32 / max 64 flips it to the varint length-prefixed
            // encoding, making every already-stored document undecodable.
            assert_rejected(
                platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
                platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":64,"position":0}),
            );
        }

        #[test]
        fn rejects_removing_max_items_from_fixed_byte_array() {
            // Removing `maxItems` turns a fixed (raw, no length prefix) byte array
            // into a variable (varint length-prefixed) one, so it must be rejected.
            assert_rejected(
                platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
                platform_value!({"type":"array","byteArray":true,"minItems":32,"position":0}),
            );
        }

        #[test]
        fn rejects_changing_fixed_byte_array_size() {
            // The byte-array check runs before validate_schema, so a fixed-size
            // change is caught here as an encoding change.
            assert_rejected(
                platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
                platform_value!({"type":"array","byteArray":true,"minItems":64,"maxItems":64,"position":0}),
            );
        }

        #[test]
        fn rejects_tightening_variable_to_fixed_byte_array() {
            // The reverse flip: a variable (varint length-prefixed) byte array
            // narrowed to fixed (raw) also changes the on-disk layout -- old docs
            // carry a length prefix the new fixed type would misread.
            assert_rejected(
                platform_value!({"type":"array","byteArray":true,"minItems":1,"maxItems":32,"position":0}),
                platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
            );
        }

        #[test]
        fn accepts_unchanged_fixed_byte_array() {
            assert_accepted(
                platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
                platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
            );
        }

        #[test]
        fn accepts_widening_already_variable_byte_array() {
            // Variable-length on both sides: the on-disk encoding does not change,
            // so widening the bound stays allowed.
            assert_accepted(
                platform_value!({"type":"array","byteArray":true,"minItems":1,"maxItems":32,"position":0}),
                platform_value!({"type":"array","byteArray":true,"minItems":1,"maxItems":64,"position":0}),
            );
        }

        #[test]
        fn accepts_max_items_change_when_min_items_is_omitted() {
            // With `minItems` omitted (None) the encoder always uses the variable
            // (varint length-prefixed) path regardless of `maxItems` -- the raw
            // path requires BOTH bounds present and equal. So changing `maxItems`
            // does not change the on-disk encoding and must stay allowed. An
            // implicit `minItems: 0` is NOT fixed-length (mirrors the encoder).
            assert_accepted(
                platform_value!({"type":"array","byteArray":true,"maxItems":0,"position":0}),
                platform_value!({"type":"array","byteArray":true,"maxItems":1,"position":0}),
            );
        }
    }
}
