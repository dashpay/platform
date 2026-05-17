use crate::consensus::basic::document::InvalidDocumentTransitionActionError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::nft::TradeMode;
use crate::state_transition::batch_transition::batched_transition::DocumentPurchaseTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait DocumentPurchaseTransitionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentPurchaseTransitionStructureValidationV0 for DocumentPurchaseTransition {
    fn validate_structure_v0(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Mirrors the drive-abci action validator trade-mode check; safe
        // pre-sign because it depends only on the document type definition.
        // The self-purchase check now lives in
        // `DocumentPurchaseTransition::from_document`, which has both the
        // seller (`document.owner_id()`) and the buyer (`new_owner_id`).
        if document_type.trade_mode() != TradeMode::DirectPurchase {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "{} trade mode is not direct purchase but we are trying to purchase directly",
                    document_type.name()
                ))
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
