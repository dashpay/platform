use crate::consensus::basic::data_contract::{
    DataContractInvalidIndexDefinitionUpdateError, IncompatibleDocumentTypeSchemaError,
};
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
        let result = self.validate_config(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        let result = self.validate_indices(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

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

    fn validate_indices(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        // There is no changes. All good
        if self.index_structure() == new_document_type.index_structure() {
            return SimpleConsensusValidationResult::new();
        }

        // We want to figure out what changed, so we compare one way then the other

        // If the new contract document type doesn't contain all previous indexes
        if let Some(non_subset_path) = new_document_type
            .index_structure()
            .contains_subset_first_non_subset_path(self.index_structure())
        {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    self.name().clone(),
                    non_subset_path,
                )
                .into(),
            );
        }

        // If the old contract document type doesn't contain all new indexes
        if let Some(non_subset_path) = self
            .index_structure()
            .contains_subset_first_non_subset_path(new_document_type.index_structure())
        {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    self.name().clone(),
                    non_subset_path,
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

        let old_document_schema_json = match self.schema().clone().try_into_validating_json() {
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

        let new_document_schema_json = match new_document_type
            .schema()
            .clone()
            .try_into_validating_json()
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
