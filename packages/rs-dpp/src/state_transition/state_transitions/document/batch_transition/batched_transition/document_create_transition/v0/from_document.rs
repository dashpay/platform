use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0Getters};
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
use crate::tokens::token_payment_info::TokenPaymentInfo;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentCreateTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        entropy: [u8; 32],
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        // Once the id depends on the nonce, the id the document was given
        // when it was built (before a nonce was assigned) is a placeholder:
        // the transition carries the id consensus will recompute.
        let mut document = document;
        if Document::document_id_depends_on_nonce(platform_version)? {
            document.set_id_for_creation(
                document_type,
                &entropy,
                identity_contract_nonce,
                platform_version,
            )?;
        }
        let prefunded_voting_balance =
            document_type.prefunded_voting_balance_for_document(&document, platform_version)?;
        Ok(DocumentCreateTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document,
                document_type,
                token_payment_info,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?,
            entropy,
            data: document.properties_consumed(),
            prefunded_voting_balance,
        })
    }
}
