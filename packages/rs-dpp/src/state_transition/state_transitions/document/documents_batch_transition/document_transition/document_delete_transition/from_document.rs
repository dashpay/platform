use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::IdentityContractNonce;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

use crate::state_transition::documents_batch_transition::document_transition::{DocumentDeleteTransition};
use crate::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;

impl DocumentDeleteTransition {
    pub fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        identity_contract_nonce: IdentityContractNonce,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .document_delete_state_transition
                .bounds
                .default_current_version,
        ) {
            0 => Ok(DocumentDeleteTransitionV0::from_document(
                document,
                document_type,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentDeleteTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
