#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "state-transition-signing")]
use crate::document::{Document, DocumentV0Getters};
use crate::fee::Credits;
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
use crate::state_transition::documents_batch_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::documents_batch_transition::document_transition::{
    DocumentReplaceTransition, DocumentTransferTransition, DocumentPurchaseTransition, DocumentUpdatePriceTransition,
};
use crate::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::documents_batch_transition::DocumentsBatchTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::documents_batch_transition::{
    DocumentDeleteTransition, DocumentsBatchTransition,
};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};
use crate::state_transition::documents_batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;

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
        replace_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let replace_transition = DocumentReplaceTransition::from_document(
            document,
            document_type,
            identity_contract_nonce,
            platform_version,
            replace_feature_version,
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

    #[cfg(feature = "state-transition-signing")]
    fn new_document_transfer_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        recipient_owner_id: Identifier,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        transfer_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let transfer_transition = DocumentTransferTransition::from_document(
            document,
            document_type,
            identity_contract_nonce,
            recipient_owner_id,
            platform_version,
            transfer_feature_version,
            base_feature_version,
        )?;
        let documents_batch_transition: DocumentsBatchTransition = DocumentsBatchTransitionV0 {
            owner_id,
            transitions: vec![transfer_transition.into()],
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
    fn new_document_deletion_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        delete_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let delete_transition = DocumentDeleteTransition::from_document(
            document,
            document_type,
            identity_contract_nonce,
            platform_version,
            delete_feature_version,
            base_feature_version,
        )?;
        let documents_batch_transition: DocumentsBatchTransition = DocumentsBatchTransitionV0 {
            owner_id,
            transitions: vec![delete_transition.into()],
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
    fn new_document_update_price_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        update_price_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let transfer_transition = DocumentUpdatePriceTransition::from_document(
            document,
            document_type,
            price,
            identity_contract_nonce,
            platform_version,
            update_price_feature_version,
            base_feature_version,
        )?;
        let documents_batch_transition: DocumentsBatchTransition = DocumentsBatchTransitionV0 {
            owner_id,
            transitions: vec![transfer_transition.into()],
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
    fn new_document_purchase_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        new_owner_id: Identifier,
        price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        purchase_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let purchase_transition = DocumentPurchaseTransition::from_document(
            document,
            document_type,
            price,
            identity_contract_nonce,
            platform_version,
            purchase_feature_version,
            base_feature_version,
        )?;
        let documents_batch_transition: DocumentsBatchTransition = DocumentsBatchTransitionV0 {
            owner_id: new_owner_id,
            transitions: vec![purchase_transition.into()],
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

    fn all_purchases_amount(&self) -> Result<Option<Credits>, ProtocolError> {
        let (total, any_purchases): (Option<Credits>, bool) = self
            .transitions
            .iter()
            .filter_map(|transition| {
                transition
                    .as_transition_purchase()
                    .map(|purchase| purchase.price())
            })
            .fold((None, false), |(acc, _), price| match acc {
                Some(acc_val) => acc_val
                    .checked_add(price)
                    .map_or((None, true), |sum| (Some(sum), true)),
                None => (Some(price), true),
            });

        match (total, any_purchases) {
            (Some(total), _) => Ok(Some(total)),
            (None, true) => Err(ProtocolError::Overflow("overflow in all purchases amount")), // Overflow occurred
            _ => Ok(None), // No purchases were found
        }
    }

    fn all_conflicting_index_collateral_voting_funds(
        &self,
    ) -> Result<Option<Credits>, ProtocolError> {
        let (total, any_voting_funds): (Option<Credits>, bool) = self
            .transitions
            .iter()
            .filter_map(|transition| {
                transition
                    .as_transition_create()
                    .and_then(|document_create_transition| {
                        // Safely sum up values to avoid overflow.
                        document_create_transition
                            .prefunded_voting_balance()
                            .as_ref()
                            .map(|(_, credits)| *credits)
                    })
            })
            .fold((None, false), |(acc, _), price| match acc {
                Some(acc_val) => acc_val
                    .checked_add(price)
                    .map_or((None, true), |sum| (Some(sum), true)),
                None => (Some(price), true),
            });

        match (total, any_voting_funds) {
            (Some(total), _) => Ok(Some(total)),
            (None, true) => Err(ProtocolError::Overflow(
                "overflow in all voting funds amount",
            )), // Overflow occurred
            _ => Ok(None),
        }
    }
}
