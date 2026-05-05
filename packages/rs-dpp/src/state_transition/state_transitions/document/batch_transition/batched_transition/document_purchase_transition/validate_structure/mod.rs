use crate::data_contract::document_type::DocumentTypeRef;
use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::validate_structure::v0::DocumentPurchaseTransitionStructureValidationV0;
use crate::state_transition::batch_transition::batched_transition::DocumentPurchaseTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

pub(crate) trait DocumentPurchaseTransitionStructureValidation {
    fn validate_structure(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentPurchaseTransitionStructureValidation for DocumentPurchaseTransition {
    fn validate_structure(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .document_purchase_transition_structure_validation
        {
            0 => self.validate_structure_v0(document_type),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentPurchaseTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
