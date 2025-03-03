use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::errors::DocumentError;
use crate::document::{Document, DocumentV0Getters};
use crate::balances::credits::Credits;
use crate::prelude::IdentityNonce;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransitionV0;
use crate::state_transition::state_transitions::document::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use versioned_feature_core::FeatureVersion;

impl DocumentPurchaseTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        price: Credits,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        let Some(revision) = document.revision() else {
            return Err(ProtocolError::Document(Box::new(
                DocumentError::DocumentNoRevisionError {
                    document: Box::new(document.clone()),
                },
            )));
        };

        Ok(DocumentPurchaseTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document,
                document_type,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?,
            revision,
            price,
        })
    }
}
