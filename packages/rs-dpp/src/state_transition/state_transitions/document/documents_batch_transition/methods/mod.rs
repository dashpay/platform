use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::documents_batch_transition::{
    DocumentsBatchTransition, DocumentsBatchTransitionV0,
};
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

pub mod v0;

impl DocumentsBatchTransitionMethodsV0 for DocumentsBatchTransition {
    fn new_document_creation_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        entropy: [u8; 32],
        identity_public_key: &IdentityPublicKey,
        signer: &S,
        platform_version: &PlatformVersion,
        batch_feature_version: Option<FeatureVersion>,
        create_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .documents_batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                DocumentsBatchTransitionV0::new_document_creation_transition_from_document(
                    document,
                    document_type,
                    entropy,
                    identity_public_key,
                    signer,
                    platform_version,
                    batch_feature_version,
                    create_feature_version,
                    base_feature_version,
                )?
                .into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_created_from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn new_document_replacement_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        identity_public_key: &IdentityPublicKey,
        signer: &S,
        platform_version: &PlatformVersion,
        batch_feature_version: Option<FeatureVersion>,
        update_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .documents_batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                DocumentsBatchTransitionV0::new_document_replacement_transition_from_document(
                    document,
                    document_type,
                    identity_public_key,
                    signer,
                    platform_version,
                    batch_feature_version,
                    update_feature_version,
                    base_feature_version,
                )?
                .into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "DocumentsBatchTransition::new_document_replacement_transition_from_document"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
