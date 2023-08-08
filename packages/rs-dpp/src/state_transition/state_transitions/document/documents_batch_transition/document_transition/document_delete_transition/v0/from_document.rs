use platform_version::version::{FeatureVersion, PlatformVersion};
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::document::{Document, DocumentV0Getters};
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;

impl DocumentDeleteTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        Ok(DocumentDeleteTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document,
                document_type,
                platform_version,
                base_feature_version,
            )?,
        })
    }
}
