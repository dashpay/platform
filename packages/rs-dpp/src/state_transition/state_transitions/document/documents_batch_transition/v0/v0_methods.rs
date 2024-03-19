#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "state-transition-signing")]
use crate::document::{Document, DocumentV0Getters};
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::SecurityLevel;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::IdentityPublicKey;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::documents_batch_transition::document_transition::DocumentReplaceTransition;
use crate::state_transition::documents_batch_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use crate::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::documents_batch_transition::DocumentsBatchTransition;
use crate::state_transition::documents_batch_transition::DocumentsBatchTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
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
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
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
            identity_contract_nonce,
            platform_version,
            create_feature_version,
            base_feature_version,
        )?;
        let documents_batch_transition: DocumentsBatchTransition = DocumentsBatchTransitionV0 {
            owner_id,
            transitions: vec![create_transition.into()],
            user_fee_increase,
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
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
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
            identity_contract_nonce,
            platform_version,
            update_feature_version,
            base_feature_version,
        )?;
        let documents_batch_transition: DocumentsBatchTransition = DocumentsBatchTransitionV0 {
            owner_id,
            transitions: vec![replace_transition.into()],
            user_fee_increase,
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

    fn set_transitions(&mut self, transitions: Vec<DocumentTransition>) {
        self.transitions = transitions;
    }

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        self.transitions
            .iter_mut()
            .for_each(|transition| transition.set_identity_contract_nonce(identity_contract_nonce));
    }
}
