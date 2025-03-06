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
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::batched_transition::{
    BatchedTransition, BatchedTransitionMutRef, BatchedTransitionRef,
};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::batched_transition::{
    DocumentPurchaseTransition, DocumentReplaceTransition, DocumentTransferTransition,
    DocumentUpdatePriceTransition,
};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
use crate::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use std::iter::Map;
use std::slice::Iter;

use crate::state_transition::batch_transition::{BatchTransitionV1, TokenBurnTransition, TokenConfigUpdateTransition, TokenDestroyFrozenFundsTransition, TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition, TokenTransferTransition, TokenUnfreezeTransition};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::{
    BatchTransition, DocumentDeleteTransition,
};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};
#[cfg(feature = "state-transition-signing")]
use crate::balances::credits::TokenAmount;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
#[cfg(feature = "state-transition-signing")]
use crate::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
use crate::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_burn_transition::TokenBurnTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_config_update_transition::TokenConfigUpdateTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_emergency_action_transition::TokenEmergencyActionTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_freeze_transition::TokenFreezeTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_transfer_transition::TokenTransferTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_unfreeze_transition::TokenUnfreezeTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::emergency_action::TokenEmergencyAction;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};

impl DocumentsBatchTransitionAccessorsV0 for BatchTransitionV1 {
    type IterType<'a>
        = Map<Iter<'a, BatchedTransition>, fn(&'a BatchedTransition) -> BatchedTransitionRef<'a>>
    where
        Self: 'a;

    /// Iterator for `BatchedTransitionRef` items in version 1.
    fn transitions_iter<'a>(&'a self) -> Self::IterType<'a> {
        self.transitions
            .iter()
            .map(|transition| transition.borrow_as_ref())
    }

    /// Returns the total number of transitions (document and token) in version 1.
    fn transitions_len(&self) -> usize {
        self.transitions.len()
    }

    /// Checks if there are no transitions in version 1.
    fn transitions_are_empty(&self) -> bool {
        self.transitions.is_empty()
    }

    /// Returns the first transition, if it exists, as a `BatchedTransitionRef`.
    fn first_transition(&self) -> Option<BatchedTransitionRef> {
        self.transitions
            .first()
            .map(|transition| transition.borrow_as_ref())
    }

    /// Returns the first transition, if it exists, as a `BatchedTransitionMutRef`.
    fn first_transition_mut(&mut self) -> Option<BatchedTransitionMutRef> {
        self.transitions
            .first_mut()
            .map(|transition| transition.borrow_as_mut())
    }
}

impl DocumentsBatchTransitionMethodsV0 for BatchTransitionV1 {
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
        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Document(create_transition.into())],
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
        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Document(replace_transition.into())],
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
        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Document(transfer_transition.into())],
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
        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Document(delete_transition.into())],
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
        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Document(transfer_transition.into())],
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
        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id: new_owner_id,
            transitions: vec![BatchedTransition::Document(purchase_transition.into())],
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

    fn set_transitions(&mut self, transitions: Vec<BatchedTransition>) {
        self.transitions = transitions;
    }

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        self.transitions
            .iter_mut()
            .for_each(|transition| transition.set_identity_contract_nonce(identity_contract_nonce));
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

impl DocumentsBatchTransitionMethodsV1 for BatchTransitionV1 {
    #[cfg(feature = "state-transition-signing")]
    fn new_token_mint_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        amount: TokenAmount,
        issued_to_identity_id: Option<Identifier>,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        _delete_feature_version: Option<FeatureVersion>,
        _base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut mint_transition = TokenMintTransition::V0(TokenMintTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce,
                token_contract_position,
                data_contract_id,
                token_id,
                using_group_info: None,
            }),
            issued_to_identity_id,
            amount,
            public_note,
        });

        if let Some(using_group_info_status) = using_group_info {
            match using_group_info_status {
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                    group_contract_position,
                ) => {
                    let action_id = mint_transition.calculate_action_id(owner_id);
                    mint_transition.base_mut().set_using_group_info(Some(
                        GroupStateTransitionInfo {
                            group_contract_position,
                            action_id,
                            action_is_proposer: true,
                        },
                    ))
                }
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                    mint_transition.base_mut().set_using_group_info(Some(info))
                }
            }
        }

        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(mint_transition.into())],
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
    fn new_token_burn_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        amount: TokenAmount,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        _delete_feature_version: Option<FeatureVersion>,
        _base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut burn_transition = TokenBurnTransition::V0(TokenBurnTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce,
                token_contract_position,
                data_contract_id,
                token_id,
                using_group_info: None,
            }),
            burn_amount: amount,
            public_note,
        });

        if let Some(using_group_info_status) = using_group_info {
            match using_group_info_status {
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                    group_contract_position,
                ) => {
                    let action_id = burn_transition.calculate_action_id(owner_id);
                    burn_transition.base_mut().set_using_group_info(Some(
                        GroupStateTransitionInfo {
                            group_contract_position,
                            action_id,
                            action_is_proposer: true,
                        },
                    ))
                }
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                    burn_transition.base_mut().set_using_group_info(Some(info))
                }
            }
        }

        // Wrap in a batch transition
        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(burn_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        // Create the state transition
        let mut state_transition: StateTransition = documents_batch_transition.into();
        state_transition.sign_external(
            identity_public_key,
            signer,
            Some(|_, _| Ok(SecurityLevel::HIGH)),
        )?;

        Ok(state_transition)
    }
    #[cfg(feature = "state-transition-signing")]
    fn new_token_transfer_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        amount: TokenAmount,
        recipient_id: Identifier,
        public_note: Option<String>,
        shared_encrypted_note: Option<SharedEncryptedNote>,
        private_encrypted_note: Option<PrivateEncryptedNote>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        _delete_feature_version: Option<FeatureVersion>,
        _base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        // Create the transfer transition for batch version 1
        let transfer_transition = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce,
                token_contract_position,
                data_contract_id,
                token_id,
                using_group_info: None,
            }),
            recipient_id,
            amount,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
        });

        // Wrap in a batch transition
        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(transfer_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        // Create the state transition
        let mut state_transition: StateTransition = documents_batch_transition.into();
        state_transition.sign_external(
            identity_public_key,
            signer,
            Some(|_, _| Ok(SecurityLevel::HIGH)),
        )?;

        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_token_freeze_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        _delete_feature_version: Option<FeatureVersion>,
        _base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut freeze_transition = TokenFreezeTransition::V0(TokenFreezeTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce,
                token_contract_position,
                data_contract_id,
                token_id,
                using_group_info: None,
            }),
            identity_to_freeze_id: frozen_identity_id,
            public_note,
        });

        if let Some(using_group_info_status) = using_group_info {
            match using_group_info_status {
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                    group_contract_position,
                ) => {
                    let action_id = freeze_transition.calculate_action_id(owner_id);
                    freeze_transition.base_mut().set_using_group_info(Some(
                        GroupStateTransitionInfo {
                            group_contract_position,
                            action_id,
                            action_is_proposer: true,
                        },
                    ))
                }
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                    freeze_transition
                        .base_mut()
                        .set_using_group_info(Some(info))
                }
            }
        }

        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(freeze_transition.into())],
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
    fn new_token_unfreeze_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        _delete_feature_version: Option<FeatureVersion>,
        _base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut unfreeze_transition = TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce,
                token_contract_position,
                data_contract_id,
                token_id,
                using_group_info: None,
            }),
            frozen_identity_id,
            public_note,
        });

        if let Some(using_group_info_status) = using_group_info {
            match using_group_info_status {
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                    group_contract_position,
                ) => {
                    let action_id = unfreeze_transition.calculate_action_id(owner_id);
                    unfreeze_transition.base_mut().set_using_group_info(Some(
                        GroupStateTransitionInfo {
                            group_contract_position,
                            action_id,
                            action_is_proposer: true,
                        },
                    ))
                }
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                    unfreeze_transition
                        .base_mut()
                        .set_using_group_info(Some(info))
                }
            }
        }

        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(unfreeze_transition.into())],
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
    fn new_token_destroy_frozen_funds_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        _delete_feature_version: Option<FeatureVersion>,
        _base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut destroy_frozen_funds_transition =
            TokenDestroyFrozenFundsTransition::V0(TokenDestroyFrozenFundsTransitionV0 {
                base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                    identity_contract_nonce,
                    token_contract_position,
                    data_contract_id,
                    token_id,
                    using_group_info: None,
                }),
                frozen_identity_id,
                public_note,
            });

        if let Some(using_group_info_status) = using_group_info {
            match using_group_info_status {
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                    group_contract_position,
                ) => {
                    let action_id = destroy_frozen_funds_transition.calculate_action_id(owner_id);
                    destroy_frozen_funds_transition
                        .base_mut()
                        .set_using_group_info(Some(GroupStateTransitionInfo {
                            group_contract_position,
                            action_id,
                            action_is_proposer: true,
                        }))
                }
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                    destroy_frozen_funds_transition
                        .base_mut()
                        .set_using_group_info(Some(info))
                }
            }
        }

        let batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(
                destroy_frozen_funds_transition.into(),
            )],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = batch_transition.into();
        state_transition.sign_external(
            identity_public_key,
            signer,
            Some(|_, _| Ok(SecurityLevel::HIGH)),
        )?;
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_token_emergency_action_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        emergency_action: TokenEmergencyAction,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        _delete_feature_version: Option<FeatureVersion>,
        _base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut emergency_action_transition =
            TokenEmergencyActionTransition::V0(TokenEmergencyActionTransitionV0 {
                base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                    identity_contract_nonce,
                    token_contract_position,
                    data_contract_id,
                    token_id,
                    using_group_info: None,
                }),
                emergency_action,
                public_note,
            });

        if let Some(using_group_info_status) = using_group_info {
            match using_group_info_status {
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                    group_contract_position,
                ) => {
                    let action_id = emergency_action_transition.calculate_action_id(owner_id);
                    emergency_action_transition
                        .base_mut()
                        .set_using_group_info(Some(GroupStateTransitionInfo {
                            group_contract_position,
                            action_id,
                            action_is_proposer: true,
                        }))
                }
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                    emergency_action_transition
                        .base_mut()
                        .set_using_group_info(Some(info))
                }
            }
        }

        let batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(emergency_action_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = batch_transition.into();
        state_transition.sign_external(
            identity_public_key,
            signer,
            Some(|_, _| Ok(SecurityLevel::HIGH)),
        )?;
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_token_config_update_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        update_token_configuration_item: TokenConfigurationChangeItem,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _batch_feature_version: Option<FeatureVersion>,
        _config_update_feature_version: Option<FeatureVersion>,
        _base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut config_update_transition =
            TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
                base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                    identity_contract_nonce,
                    token_contract_position,
                    data_contract_id,
                    token_id,
                    using_group_info: None,
                }),
                update_token_configuration_item,
                public_note,
            });

        if let Some(using_group_info_status) = using_group_info {
            match using_group_info_status {
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                    group_contract_position,
                ) => {
                    let action_id = config_update_transition.calculate_action_id(owner_id);
                    config_update_transition
                        .base_mut()
                        .set_using_group_info(Some(GroupStateTransitionInfo {
                            group_contract_position,
                            action_id,
                            action_is_proposer: true,
                        }))
                }
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                    config_update_transition
                        .base_mut()
                        .set_using_group_info(Some(info))
                }
            }
        }

        let batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(config_update_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = batch_transition.into();
        state_transition.sign_external(
            identity_public_key,
            signer,
            Some(|_, _| Ok(SecurityLevel::CRITICAL)),
        )?;
        Ok(state_transition)
    }
}
