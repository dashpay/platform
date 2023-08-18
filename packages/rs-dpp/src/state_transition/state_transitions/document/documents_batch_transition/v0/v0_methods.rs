use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0Getters};
use crate::identity::signer::Signer;
use crate::identity::SecurityLevel;
use crate::prelude::IdentityPublicKey;
use crate::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
use crate::state_transition::documents_batch_transition::document_transition::{
    DocumentReplaceTransition, DocumentTransition,
};
use crate::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::documents_batch_transition::{
    DocumentsBatchTransition, DocumentsBatchTransitionV0,
};
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentsBatchTransitionAccessorsV0 for DocumentsBatchTransitionV0 {
    fn transitions(&self) -> &Vec<DocumentTransition> {
        &self.transitions
    }

    fn transitions_slice(&self) -> &[DocumentTransition] {
        self.transitions.as_slice()
    }
}

impl DocumentsBatchTransitionMethodsV0 for DocumentsBatchTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn new_document_creation_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        entropy: [u8; 32],
        identity_public_key: &IdentityPublicKey,
        signer: &S,
        platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        create_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let create_transition = DocumentCreateTransition::from_document(
            document,
            document_type,
            entropy,
            platform_version,
            create_feature_version,
            base_feature_version,
        )?;
        let documents_batch_transition: DocumentsBatchTransition = DocumentsBatchTransitionV0 {
            owner_id,
            transitions: vec![create_transition.into()],
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = documents_batch_transition.into();
        state_transition.sign_external(
            identity_public_key,
            signer,
            Some(|_, _| Ok(SecurityLevel::HIGH)),
        )?;
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_document_replacement_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        identity_public_key: &IdentityPublicKey,
        signer: &S,
        platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        update_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let replace_transition = DocumentReplaceTransition::from_document(
            document,
            document_type,
            platform_version,
            update_feature_version,
            base_feature_version,
        )?;
        let documents_batch_transition: DocumentsBatchTransition = DocumentsBatchTransitionV0 {
            owner_id,
            transitions: vec![replace_transition.into()],
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = documents_batch_transition.into();
        state_transition.sign_external(
            identity_public_key,
            signer,
            Some(|_, _| Ok(SecurityLevel::HIGH)),
        )?;
        Ok(state_transition)
    }
}
