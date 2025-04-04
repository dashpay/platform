use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::document_transfer_transition::{
    DocumentTransferTransition, DocumentTransferTransitionV0,
};
use crate::tokens::token_payment_info::TokenPaymentInfo;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentTransferTransition {
    #[allow(clippy::too_many_arguments)]
    pub fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        recipient_owner_id: Identifier,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .document_transfer_state_transition
                .bounds
                .default_current_version,
        ) {
            0 => Ok(DocumentTransferTransitionV0::from_document(
                document,
                document_type,
                token_payment_info,
                identity_contract_nonce,
                recipient_owner_id,
                platform_version,
                base_feature_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentTransferTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
