use crate::consensus::basic::document::InvalidDocumentTransitionIdError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
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
        // Drive recomputes the document id from the entropy during
        // advanced-structure validation and rejects a mismatch with
        // InvalidDocumentTransitionIdError, after the identity contract nonce
        // has already been bumped. Refuse locally so no caller can assemble a
        // create transition that will only fail once it has been paid for.
        let expected_id = Document::generate_document_id_v0(
            &document_type.data_contract_id(),
            &document.owner_id(),
            document_type.name(),
            &entropy,
        );
        if document.id() != expected_id {
            return Err(ProtocolError::ConsensusError(Box::new(
                InvalidDocumentTransitionIdError::new(expected_id, document.id()).into(),
            )));
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
