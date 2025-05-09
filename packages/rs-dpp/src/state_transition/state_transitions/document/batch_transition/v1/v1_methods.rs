#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::IdentityPublicKey;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
use crate::state_transition::batch_transition::BatchTransitionV1;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::{TokenDirectPurchaseTransition, TokenSetPriceForDirectPurchaseTransition};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::{TokenClaimTransition, TokenBurnTransition, TokenConfigUpdateTransition, TokenDestroyFrozenFundsTransition, TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition, TokenTransferTransition, TokenUnfreezeTransition};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::BatchTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use crate::balances::credits::TokenAmount;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
#[cfg(feature = "state-transition-signing")]
use crate::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use crate::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
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
use crate::state_transition::batch_transition::token_claim_transition::TokenClaimTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_config_update_transition::TokenConfigUpdateTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_direct_purchase_transition::TokenDirectPurchaseTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_emergency_action_transition::TokenEmergencyActionTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_freeze_transition::TokenFreezeTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_transfer_transition::TokenTransferTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::token_unfreeze_transition::TokenUnfreezeTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::GetDataContractSecurityLevelRequirementFn;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::emergency_action::TokenEmergencyAction;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
#[cfg(feature = "state-transition-signing")]
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;

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
        options: Option<StateTransitionCreationOptions>,
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
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }

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
        options: Option<StateTransitionCreationOptions>,
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
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }

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
        options: Option<StateTransitionCreationOptions>,
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
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }

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
        options: Option<StateTransitionCreationOptions>,
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
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }
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
        options: Option<StateTransitionCreationOptions>,
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
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }
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
        options: Option<StateTransitionCreationOptions>,
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
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }
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
        options: Option<StateTransitionCreationOptions>,
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
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }
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
        options: Option<StateTransitionCreationOptions>,
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
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_token_claim_transition<S: Signer>(
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
        _platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let claim_transition = TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce,
                token_contract_position,
                data_contract_id,
                token_id,
                using_group_info: None,
            }),
            distribution_type,
            public_note,
        });

        let batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(claim_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = batch_transition.into();
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_token_change_direct_purchase_price_transition<S: Signer>(
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
        _platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut change_direct_purchase_price_transition =
            TokenSetPriceForDirectPurchaseTransition::V0(
                TokenSetPriceForDirectPurchaseTransitionV0 {
                    base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                        identity_contract_nonce,
                        token_contract_position,
                        data_contract_id,
                        token_id,
                        using_group_info: None,
                    }),
                    price: token_pricing_schedule,
                    public_note,
                },
            );

        if let Some(using_group_info_status) = using_group_info {
            match using_group_info_status {
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                    group_contract_position,
                ) => {
                    let action_id =
                        change_direct_purchase_price_transition.calculate_action_id(owner_id);
                    change_direct_purchase_price_transition
                        .base_mut()
                        .set_using_group_info(Some(GroupStateTransitionInfo {
                            group_contract_position,
                            action_id,
                            action_is_proposer: true,
                        }))
                }
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                    change_direct_purchase_price_transition
                        .base_mut()
                        .set_using_group_info(Some(info))
                }
            }
        }

        let batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(
                change_direct_purchase_price_transition.into(),
            )],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = batch_transition.into();
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }
        Ok(state_transition)
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_token_direct_purchase_transition<S: Signer>(
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
        _platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let direct_purchase_transition =
            TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0 {
                base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                    identity_contract_nonce,
                    token_contract_position,
                    data_contract_id,
                    token_id,
                    using_group_info: None,
                }),
                token_count: amount,
                total_agreed_price,
            });

        let batch_transition: BatchTransition = BatchTransitionV1 {
            owner_id,
            transitions: vec![BatchedTransition::Token(direct_purchase_transition.into())],
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = batch_transition.into();
        if let Some(options) = options {
            state_transition.sign_external_with_options(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
                options.signing_options,
            )?;
        } else {
            state_transition.sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
        }
        Ok(state_transition)
    }
}
