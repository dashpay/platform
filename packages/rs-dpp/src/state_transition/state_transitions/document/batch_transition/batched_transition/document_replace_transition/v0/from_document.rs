use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::errors::DocumentError;
use crate::document::{Document, DocumentV0Getters};
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::document_replace_transition::DocumentReplaceTransitionV0;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentReplaceTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        // The transition carries every `generatedFrom` property as the platform generates
        // it from the document's params, replacing a value the document holds: a document
        // fetched and edited still holds the one generated from its old params, which the
        // platform refuses. Inert before protocol version 14: the
        // `fill_generated_properties` slot is `None` there and leaves the document as it is.
        let mut document = document;
        document_type
            .regenerate_generated_properties(document.properties_mut(), platform_version)?;
        Ok(DocumentReplaceTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document,
                document_type,
                token_payment_info,
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
