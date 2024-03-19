use platform_version::version::{FeatureVersion, PlatformVersion};
use crate::data_contract::document_type::{DocumentTypeRef};
use crate::document::{Document, DocumentV0Getters};
use crate::document::errors::DocumentError;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;

impl DocumentReplaceTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        Ok(DocumentReplaceTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document,
                document_type,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?,
            revision: document.revision().ok_or_else(|| {
                ProtocolError::Document(Box::new(DocumentError::DocumentNoRevisionError {
                    document: Box::new(document.clone()),
                }))
            })?,
            data: document.properties_consumed(),
        })
    }
}
