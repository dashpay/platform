#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "state-transition-signing")]
use crate::document::{Document, DocumentV0Getters};
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::IdentityPublicKey;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::batched_transition::BatchedTransitionV1;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::batched_transition::{
    DocumentPurchaseTransition, DocumentReplaceTransition, DocumentTransferTransition,
    DocumentUpdatePriceTransition,
};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
use crate::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::batch_transition::methods::v2::DocumentsBatchTransitionMethodsV2;

use crate::state_transition::batch_transition::BatchTransitionV2;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::{
    BatchTransition, DocumentDeleteTransition, DocumentEraseTransition,
    DocumentIndexOnlyDeleteTransition,
};
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::batched_transition::DocumentTransitionV1;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::token_payment_info::TokenPaymentInfo;

impl DocumentsBatchTransitionMethodsV0 for BatchTransitionV2 {
    #[cfg(feature = "state-transition-signing")]
    async fn new_document_creation_transition_from_document<S: Signer<IdentityPublicKey>>(
        document: Document,
        document_type: DocumentTypeRef<'_>,
        entropy: [u8; 32],
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let resolved_options = options.unwrap_or_default();
        let create_transition = DocumentCreateTransition::from_document(
            document,
            document_type,
            entropy,
            token_payment_info,
            identity_contract_nonce,
            platform_version,
            resolved_options.method_feature_version,
            resolved_options.base_feature_version,
        )?;
        let documents_batch_transition: BatchTransition = BatchTransitionV2 {
            owner_id,
            transitions: vec![BatchedTransitionV1::Document(create_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = documents_batch_transition.into();
        let required_security_level = document_type.security_level_requirement();
        state_transition
            .sign_external_with_options(
                identity_public_key,
                signer,
                Some(|_, _| Ok(required_security_level)),
                resolved_options.signing_options,
            )
            .await?;
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_document_replacement_transition_from_document<S: Signer<IdentityPublicKey>>(
        document: Document,
        document_type: DocumentTypeRef<'_>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let resolved_options = options.unwrap_or_default();
        let replace_transition = DocumentReplaceTransition::from_document(
            document,
            document_type,
            token_payment_info,
            identity_contract_nonce,
            platform_version,
            resolved_options.method_feature_version,
            resolved_options.base_feature_version,
        )?;
        let documents_batch_transition: BatchTransition = BatchTransitionV2 {
            owner_id,
            transitions: vec![BatchedTransitionV1::Document(replace_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = documents_batch_transition.into();
        let required_security_level = document_type.security_level_requirement();
        state_transition
            .sign_external_with_options(
                identity_public_key,
                signer,
                Some(|_, _| Ok(required_security_level)),
                resolved_options.signing_options,
            )
            .await?;
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_document_deletion_transition_from_document<S: Signer<IdentityPublicKey>>(
        document: Document,
        document_type: DocumentTypeRef<'_>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let resolved_options = options.unwrap_or_default();
        // Deleting an indexOnly document is a different OPERATION from
        // deleting a stored one (delete-by-values vs delete-by-id), so the
        // factory picks the transition kind from the doctype's storage
        // mode — every construction path, SDKs included, produces the kind
        // the ABCI structure gates require, with no per-client knowledge
        // of the storage layout.
        let delete_transition: DocumentTransitionV1 = if document_type.index_only() {
            DocumentIndexOnlyDeleteTransition::from_document(
                document,
                document_type,
                token_payment_info,
                identity_contract_nonce,
                platform_version,
                resolved_options.method_feature_version,
                resolved_options.base_feature_version,
            )?
            .into()
        } else {
            DocumentDeleteTransition::from_document(
                document,
                document_type,
                token_payment_info,
                identity_contract_nonce,
                platform_version,
                resolved_options.method_feature_version,
                resolved_options.base_feature_version,
            )?
            .into()
        };
        let documents_batch_transition: BatchTransition = BatchTransitionV2 {
            owner_id,
            transitions: vec![BatchedTransitionV1::Document(delete_transition)],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = documents_batch_transition.into();
        let required_security_level = document_type.security_level_requirement();
        state_transition
            .sign_external_with_options(
                identity_public_key,
                signer,
                Some(|_, _| Ok(required_security_level)),
                resolved_options.signing_options,
            )
            .await?;
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    async fn new_document_transfer_transition_from_document<S: Signer<IdentityPublicKey>>(
        document: Document,
        document_type: DocumentTypeRef<'_>,
        recipient_owner_id: Identifier,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let resolved_options = options.unwrap_or_default();
        let transfer_transition = DocumentTransferTransition::from_document(
            document,
            document_type,
            token_payment_info,
            identity_contract_nonce,
            recipient_owner_id,
            platform_version,
            resolved_options.method_feature_version,
            resolved_options.base_feature_version,
        )?;
        let documents_batch_transition: BatchTransition = BatchTransitionV2 {
            owner_id,
            transitions: vec![BatchedTransitionV1::Document(transfer_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = documents_batch_transition.into();
        let required_security_level = document_type.security_level_requirement();
        state_transition
            .sign_external_with_options(
                identity_public_key,
                signer,
                Some(|_, _| Ok(required_security_level)),
                resolved_options.signing_options,
            )
            .await?;
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_document_update_price_transition_from_document<S: Signer<IdentityPublicKey>>(
        document: Document,
        document_type: DocumentTypeRef<'_>,
        price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let resolved_options = options.unwrap_or_default();
        let transfer_transition = DocumentUpdatePriceTransition::from_document(
            document,
            document_type,
            price,
            token_payment_info,
            identity_contract_nonce,
            platform_version,
            resolved_options.method_feature_version,
            resolved_options.base_feature_version,
        )?;
        let documents_batch_transition: BatchTransition = BatchTransitionV2 {
            owner_id,
            transitions: vec![BatchedTransitionV1::Document(transfer_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = documents_batch_transition.into();
        let required_security_level = document_type.security_level_requirement();
        state_transition
            .sign_external_with_options(
                identity_public_key,
                signer,
                Some(|_, _| Ok(required_security_level)),
                resolved_options.signing_options,
            )
            .await?;
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_document_purchase_transition_from_document<S: Signer<IdentityPublicKey>>(
        document: Document,
        document_type: DocumentTypeRef<'_>,
        new_owner_id: Identifier,
        price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        let purchase_transition = DocumentPurchaseTransition::from_document(
            document,
            document_type,
            price,
            token_payment_info,
            identity_contract_nonce,
            platform_version,
            resolved_options.method_feature_version,
            resolved_options.base_feature_version,
        )?;
        let documents_batch_transition: BatchTransition = BatchTransitionV2 {
            owner_id: new_owner_id,
            transitions: vec![BatchedTransitionV1::Document(purchase_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = documents_batch_transition.into();
        let required_security_level = document_type.security_level_requirement();
        state_transition
            .sign_external_with_options(
                identity_public_key,
                signer,
                Some(|_, _| Ok(required_security_level)),
                resolved_options.signing_options,
            )
            .await?;
        Ok(state_transition)
    }

    fn set_transitions(&mut self, transitions: Vec<BatchedTransition>) {
        self.transitions = transitions.into_iter().map(Into::into).collect();
    }

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        self.transitions
            .iter_mut()
            .for_each(|transition| transition.set_identity_contract_nonce(identity_contract_nonce));
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

    fn all_document_purchases_amount(&self) -> Result<Option<Credits>, ProtocolError> {
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
}

impl DocumentsBatchTransitionMethodsV2 for BatchTransitionV2 {
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    async fn new_document_erase_transition_from_document<S: Signer<IdentityPublicKey>>(
        document: Document,
        document_type: DocumentTypeRef<'_>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        options: Option<StateTransitionCreationOptions>,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let owner_id = document.owner_id();
        let resolved_options = options.unwrap_or_default();
        let erase_transition: DocumentTransitionV1 = DocumentEraseTransition::from_document(
            document,
            document_type,
            None,
            identity_contract_nonce,
            resolved_options.method_feature_version,
            resolved_options.base_feature_version,
            platform_version,
        )?
        .into();
        let documents_batch_transition: BatchTransition = BatchTransitionV2 {
            owner_id,
            transitions: vec![BatchedTransitionV1::Document(erase_transition)],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = documents_batch_transition.into();
        let required_security_level = document_type.security_level_requirement();
        state_transition
            .sign_external_with_options(
                identity_public_key,
                signer,
                Some(|_, _| Ok(required_security_level)),
                resolved_options.signing_options,
            )
            .await?;
        Ok(state_transition)
    }
}
