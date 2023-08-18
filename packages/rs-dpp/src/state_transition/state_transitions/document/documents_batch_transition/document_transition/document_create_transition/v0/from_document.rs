use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0Getters};
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentCreateTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        entropy: [u8; 32],
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        Ok(DocumentCreateTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document,
                document_type,
                platform_version,
                base_feature_version,
            )?,
            entropy,
            created_at: document.created_at(),
            updated_at: document.updated_at(),
            data: document.properties_consumed(),
        })
    }
}
