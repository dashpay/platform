use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use crate::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentBaseTransition {
    #[allow(clippy::too_many_arguments)]
    pub fn from_document(
        document: &Document,
        document_type: DocumentTypeRef,
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .document_base_state_transition
                .default_current_version,
        ) {
            0 => Ok(DocumentBaseTransitionV0::from_document(
                document,
                document_type,
                identity_contract_nonce,
            )
            .into()),
            1 => Ok(DocumentBaseTransitionV1::from_document(
                document,
                document_type,
                token_payment_info,
                identity_contract_nonce,
            )
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentBaseTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
