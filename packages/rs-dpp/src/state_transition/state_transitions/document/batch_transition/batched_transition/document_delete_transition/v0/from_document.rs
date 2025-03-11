use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::IdentityNonce;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransitionV0;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_base_transition::DocumentBaseTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use versioned_feature_core::FeatureVersion;

impl DocumentDeleteTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        Ok(DocumentDeleteTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document,
                document_type,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?,
        })
    }
}
