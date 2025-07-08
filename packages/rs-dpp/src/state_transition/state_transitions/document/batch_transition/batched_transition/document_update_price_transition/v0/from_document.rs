use platform_version::version::{FeatureVersion, PlatformVersion};
use crate::data_contract::document_type::{DocumentTypeRef};
use crate::document::{Document, DocumentV0Getters};
use crate::document::errors::DocumentError;
use crate::fee::Credits;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::batch_transition::batched_transition::document_update_price_transition::DocumentUpdatePriceTransitionV0;
use crate::tokens::token_payment_info::TokenPaymentInfo;

impl DocumentUpdatePriceTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        price: Credits,
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        Ok(DocumentUpdatePriceTransitionV0 {
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
            price,
        })
    }
}
