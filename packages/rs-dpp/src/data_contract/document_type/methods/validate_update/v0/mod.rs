use crate::consensus::basic::data_contract::IncompatibleDocumentTypeSchemaError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::schema::validate_schema_compatibility;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl<'a> DocumentTypeRef<'a> {
    #[inline(always)]
    pub(super) fn validate_update_v0(
        &self,
        new_document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Validate configuration
        let result = self.validate_config(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate indices
        let result = self
            .index_structure()
            .validate_update(self.name(), new_document_type.index_structure());

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate schema compatibility
        self.validate_schema(new_document_type, platform_version)
    }

    fn validate_config(
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

        SimpleConsensusValidationResult::new()
    }

    fn validate_schema(
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
    use crate::data_contract::document_type::DocumentType;
    use assert_matches::assert_matches;
    use platform_value::platform_value;
    use platform_value::Identifier;

    mod validate_config {
        use super::*;

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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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
    }

    mod validate_schema {
        use super::*;
        use crate::consensus::basic::BasicError;

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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema.clone(),
                None,
                false,
                false,
                false,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create old document type");

            let new_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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

            let old_document_type = DocumentType::try_from_schema(
                data_contract_id,
                document_type_name,
                schema.clone(),
                None,
                false,
                false,
                false,
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
                document_type_name,
                schema,
                None,
                false,
                false,
                false,
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
    }
}
