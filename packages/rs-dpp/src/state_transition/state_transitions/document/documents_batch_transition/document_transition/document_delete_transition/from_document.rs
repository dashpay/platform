use platform_version::version::{FeatureVersion, PlatformVersion};
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::document::{Document, DocumentV0Getters};
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;
use crate::state_transition::documents_batch_transition::document_transition::{DocumentCreateTransition, DocumentDeleteTransition};
use crate::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;

impl DocumentDeleteTransition {
    pub fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .document_delete_state_transition
                .bounds
                .default_current_version,
        ) {
            0 => Ok(DocumentDeleteTransitionV0::from_document(
                document,
                document_type,
                platform_version,
                base_feature_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentDeleteTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
