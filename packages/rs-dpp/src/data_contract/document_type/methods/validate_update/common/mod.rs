//! Helpers shared by every generation of `DocumentTypeRef::validate_update`
//! (`v0`, `v1`, …). Only the parts of the update-validation flow that differ
//! between generations live in the per-version modules; the config, byte-array
//! encoding and JSON-schema compatibility checks below are generation
//! independent.

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

        if new_document_type.documents_can_be_deleted() != self.documents_can_be_deleted() {
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

        SimpleConsensusValidationResult::new()
    }

    pub(super) fn validate_schema(
        &self,
        new_document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // All good if schema is the same
        if self.schema() == new_document_type.schema() {
            return Ok(SimpleConsensusValidationResult::new());
        }

        let old_document_schema_json = match self.schema().try_to_validating_json() {
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

        let new_document_schema_json = match new_document_type.schema().try_to_validating_json() {
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
                .validate_update(new.as_ref(), platform_version)
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
