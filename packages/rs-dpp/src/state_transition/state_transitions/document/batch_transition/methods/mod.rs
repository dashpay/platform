#[cfg(feature = "state-transition-signing")]
use crate::balances::credits::TokenAmount;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use crate::data_contract::document_type::action_fees::agreement::DocumentActionFeeAgreement;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::TokenContractPosition;
#[cfg(feature = "state-transition-signing")]
use crate::document::Document;
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::group::GroupStateTransitionInfoStatus;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
use crate::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use crate::state_transition::batch_transition::BatchTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::{BatchTransitionV0, BatchTransitionV1};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionSigningOptions;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::emergency_action::TokenEmergencyAction;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::token_payment_info::TokenPaymentInfo;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
use platform_version::version::FeatureVersion;
use platform_version::version::PlatformVersion;

pub mod v0;
pub mod v1;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct StateTransitionCreationOptions {
    /// The signing options
    pub signing_options: StateTransitionSigningOptions,
    pub batch_feature_version: Option<FeatureVersion>,
    pub method_feature_version: Option<FeatureVersion>,
    pub base_feature_version: Option<FeatureVersion>,
    /// The action fees the document transition agrees to pay. Required when the document type
    /// charges a fee for the action (protocol version 14).
    pub action_fee_agreement: Option<DocumentActionFeeAgreement>,
}

impl StateTransitionCreationOptions {
    /// Fails when the options name an action fee agreement and the document base they select
    /// under `platform_version` is too old to carry it, which the builders would only find out
    /// after their caller has spent an identity contract nonce on the transition. A caller that
    /// reserves the nonce first asks this before it does.
    pub fn validate_base_carries_action_fee_agreement(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        if self.action_fee_agreement.is_none() {
            return Ok(());
        }
        let base_version = self.base_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .document_base_state_transition
                .default_current_version,
        );
        if base_version < 2 {
            return Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "StateTransitionCreationOptions::validate_base_carries_action_fee_agreement"
                        .to_string(),
                known_versions: vec![2],
                received: base_version,
            });
        }
        Ok(())
    }

    /// `transition` carrying the options' action fee agreement, if they name one. A base too
    /// old to carry it is an error: the transition must not go out without what its signer
    /// agreed to.
    pub fn apply_action_fee_agreement(
        &self,
        transition: impl Into<DocumentTransition>,
    ) -> Result<DocumentTransition, ProtocolError> {
        let mut transition = transition.into();
        if let Some(action_fee_agreement) = self.action_fee_agreement {
            transition
                .base_mut()
                .try_set_action_fee_agreement(action_fee_agreement)?;
        }
        Ok(transition)
    }
}

impl DocumentsBatchTransitionMethodsV0 for BatchTransition {
    fn all_document_purchases_amount(&self) -> Result<Option<Credits>, ProtocolError> {
        match self {
            BatchTransition::V0(v0) => v0.all_document_purchases_amount(),
            BatchTransition::V1(v1) => v1.all_document_purchases_amount(),
        }
    }

    fn all_conflicting_index_collateral_voting_funds(
        &self,
    ) -> Result<Option<Credits>, ProtocolError> {
        match self {
            BatchTransition::V0(v0) => v0.all_conflicting_index_collateral_voting_funds(),
            BatchTransition::V1(v1) => v1.all_conflicting_index_collateral_voting_funds(),
        }
    }

    fn set_transitions(&mut self, transitions: Vec<BatchedTransition>) {
        match self {
            BatchTransition::V0(v0) => v0.set_transitions(transitions),
            BatchTransition::V1(v1) => v1.set_transitions(transitions),
        }
    }

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        match self {
            BatchTransition::V0(v0) => v0.set_identity_contract_nonce(identity_contract_nonce),
            BatchTransition::V1(v1) => v1.set_identity_contract_nonce(identity_contract_nonce),
        }
    }

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
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                BatchTransitionV0::new_document_creation_transition_from_document(
                    document,
                    document_type,
                    entropy,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            1 => Ok(
                BatchTransitionV1::new_document_creation_transition_from_document(
                    document,
                    document_type,
                    entropy,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_created_from_document".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
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
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                BatchTransitionV0::new_document_replacement_transition_from_document(
                    document,
                    document_type,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            1 => Ok(
                BatchTransitionV1::new_document_replacement_transition_from_document(
                    document,
                    document_type,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "DocumentsBatchTransition::new_document_replacement_transition_from_document"
                        .to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
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
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                BatchTransitionV0::new_document_transfer_transition_from_document(
                    document,
                    document_type,
                    recipient_owner_id,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            1 => Ok(
                BatchTransitionV1::new_document_transfer_transition_from_document(
                    document,
                    document_type,
                    recipient_owner_id,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "DocumentsBatchTransition::new_document_replacement_transition_from_document"
                        .to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
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
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                BatchTransitionV0::new_document_deletion_transition_from_document(
                    document,
                    document_type,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            1 => Ok(
                BatchTransitionV1::new_document_deletion_transition_from_document(
                    document,
                    document_type,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_document_deletion_transition_from_document"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
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
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                BatchTransitionV0::new_document_update_price_transition_from_document(
                    document,
                    document_type,
                    price,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            1 => Ok(
                BatchTransitionV1::new_document_update_price_transition_from_document(
                    document,
                    document_type,
                    price,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "DocumentsBatchTransition::new_document_update_price_transition_from_document"
                        .to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
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
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                BatchTransitionV0::new_document_purchase_transition_from_document(
                    document,
                    document_type,
                    new_owner_id,
                    price,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            1 => Ok(
                BatchTransitionV1::new_document_purchase_transition_from_document(
                    document,
                    document_type,
                    new_owner_id,
                    price,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    token_payment_info,
                    signer,
                    platform_version,
                    options,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_document_purchase_transition_from_document"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

impl DocumentsBatchTransitionMethodsV1 for BatchTransition {
    #[cfg(feature = "state-transition-signing")]
    async fn new_token_mint_transition<S: Signer<IdentityPublicKey>>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        amount: TokenAmount,
        issued_to_identity_id: Option<Identifier>,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                BatchTransitionV1::new_token_mint_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    amount,
                    issued_to_identity_id,
                    public_note,
                    using_group_info,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_mint_transition".to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_token_burn_transition<S: Signer<IdentityPublicKey>>(
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
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                BatchTransitionV1::new_token_burn_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    amount,
                    public_note,
                    using_group_info,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_burn_transition".to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_token_transfer_transition<S: Signer<IdentityPublicKey>>(
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
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                // Create the transfer transition for batch version 1
                BatchTransitionV1::new_token_transfer_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    amount,
                    recipient_id,
                    public_note,
                    shared_encrypted_note,
                    private_encrypted_note,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_transfer_transition".to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_token_freeze_transition<S: Signer<IdentityPublicKey>>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        freeze_identity_id: Identifier,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                // Create the freeze transition for batch version 1
                BatchTransitionV1::new_token_freeze_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    freeze_identity_id,
                    public_note,
                    using_group_info,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_freeze_transition".to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_token_unfreeze_transition<S: Signer<IdentityPublicKey>>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        unfreeze_identity_id: Identifier,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                // Create the freeze transition for batch version 1
                BatchTransitionV1::new_token_unfreeze_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    unfreeze_identity_id,
                    public_note,
                    using_group_info,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_unfreeze_transition".to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_token_destroy_frozen_funds_transition<S: Signer<IdentityPublicKey>>(
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
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                // Create the destroy frozen funds transition for batch version 1
                BatchTransitionV1::new_token_destroy_frozen_funds_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    frozen_identity_id,
                    public_note,
                    using_group_info,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_destroy_frozen_funds_transition"
                    .to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_token_emergency_action_transition<S: Signer<IdentityPublicKey>>(
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
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                // Create the emergency action transition for batch version 1
                BatchTransitionV1::new_token_emergency_action_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    emergency_action,
                    public_note,
                    using_group_info,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_emergency_action_transition"
                    .to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_token_config_update_transition<S: Signer<IdentityPublicKey>>(
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
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                // Create the emergency action transition for batch version 1
                BatchTransitionV1::new_token_config_update_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    update_token_configuration_item,
                    public_note,
                    using_group_info,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_config_update_transition".to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_token_claim_transition<S: Signer<IdentityPublicKey>>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        distribution_type: TokenDistributionType,
        public_note: Option<String>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                // Create the emergency action transition for batch version 1
                BatchTransitionV1::new_token_claim_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    distribution_type,
                    public_note,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_claim_transition".to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    async fn new_token_change_direct_purchase_price_transition<S: Signer<IdentityPublicKey>>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        token_pricing_schedule: Option<TokenPricingSchedule>,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                // Create the emergency action transition for batch version 1
                BatchTransitionV1::new_token_change_direct_purchase_price_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    token_pricing_schedule,
                    public_note,
                    using_group_info,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "DocumentsBatchTransition::new_token_change_direct_purchase_price_transition"
                        .to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    async fn new_token_direct_purchase_transition<S: Signer<IdentityPublicKey>>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        amount: TokenAmount,
        total_agreed_price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let resolved_options = options.unwrap_or_default();
        match resolved_options.batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .default_current_version,
        ) {
            1 | 0
                if platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .batch_state_transition
                    .max_version
                    >= 1 =>
            {
                // Create the emergency action transition for batch version 1
                BatchTransitionV1::new_token_direct_purchase_transition(
                    token_id,
                    owner_id,
                    data_contract_id,
                    token_contract_position,
                    amount,
                    total_agreed_price,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    options,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_token_direct_purchase_transition"
                    .to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod action_fee_agreement_option_tests {
    use super::*;
    use crate::data_contract::document_type::action_fees::agreement::AgreedFeeMultiplier;
    use crate::data_contract::document_type::action_fees::{ActionFeePricing, DocumentActionFee};

    fn agreeing() -> StateTransitionCreationOptions {
        StateTransitionCreationOptions {
            action_fee_agreement: Some(DocumentActionFeeAgreement::for_declared_fee(
                ActionFeePricing::Fixed,
                DocumentActionFee {
                    owner: 1,
                    moderators: 2,
                },
                AgreedFeeMultiplier {
                    known_permille: 1000,
                    increase_tolerance_percent: 0,
                },
            )),
            ..Default::default()
        }
    }

    // What a caller asks before it reserves an identity contract nonce: protocol version 13
    // builds a version 1 base, which cannot carry an agreement.
    #[test]
    fn should_refuse_an_agreement_the_selected_base_cannot_carry() {
        let version_13 = PlatformVersion::get(13).expect("expected protocol version 13");
        assert!(matches!(
            agreeing().validate_base_carries_action_fee_agreement(version_13),
            Err(ProtocolError::UnknownVersionMismatch { received: 1, .. })
        ));
        let forced_old_base = StateTransitionCreationOptions {
            base_feature_version: Some(1),
            ..agreeing()
        };
        assert!(forced_old_base
            .validate_base_carries_action_fee_agreement(PlatformVersion::latest())
            .is_err());
    }

    #[test]
    fn should_accept_an_agreement_on_a_base_that_carries_it_and_options_without_one() {
        let version_13 = PlatformVersion::get(13).expect("expected protocol version 13");
        assert!(agreeing()
            .validate_base_carries_action_fee_agreement(PlatformVersion::latest())
            .is_ok());
        assert!(StateTransitionCreationOptions::default()
            .validate_base_carries_action_fee_agreement(version_13)
            .is_ok());
    }
}
