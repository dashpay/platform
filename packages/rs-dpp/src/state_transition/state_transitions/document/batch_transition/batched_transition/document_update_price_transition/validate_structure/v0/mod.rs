use crate::consensus::basic::document::InvalidDocumentTransitionActionError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::state_transition::batch_transition::batched_transition::DocumentUpdatePriceTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait DocumentUpdatePriceTransitionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentUpdatePriceTransitionStructureValidationV0 for DocumentUpdatePriceTransition {
    fn validate_structure_v0(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Mirrors the drive-abci action validator trade-mode check; safe
        // pre-sign because it depends only on the document type definition.
        if !document_type.trade_mode().seller_sets_price() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "{} is in trade mode {} that does not support the seller setting the price",
                    document_type.name(),
                    document_type.trade_mode(),
                ))
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
