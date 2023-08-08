use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::document::{Document, DocumentV0Getters};
use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentBaseTransition {
    pub fn from_document(
        document: &Document,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .document_base_state_transition
                .default_current_version,
        ) {
            0 => Ok(DocumentBaseTransitionV0::from_document(document, document_type).into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentBaseTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
