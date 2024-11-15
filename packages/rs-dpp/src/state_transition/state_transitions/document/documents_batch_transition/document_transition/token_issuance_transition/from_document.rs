use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::IdentityNonce;
use crate::state_transition::documents_batch_transition::token_issuance_transition::TokenIssuanceTransitionV0;
use crate::state_transition::documents_batch_transition::document_transition::TokenIssuanceTransition;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl TokenIssuanceTransition {
    pub fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        entropy: [u8; 32],
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .token_issuance_state_transition
                .bounds
                .default_current_version,
        ) {
            0 => Ok(TokenIssuanceTransitionV0::from_document(
                document,
                document_type,
                entropy,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenIssuanceTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
