use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::validation::SimpleConsensusValidationResult;

impl<'a> DocumentTypeRef<'a> {
    #[inline(always)]
    pub(super) fn validate_update_v0(
        &self,
        new_document_type: &DocumentType,
    ) -> SimpleConsensusValidationResult {
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

        if new_document_type.required_fields() != self.required_fields() {
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change required fields: changing from {:?} to {:?}",
                        self.required_fields(),
                        new_document_type.required_fields()
                    ),
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
