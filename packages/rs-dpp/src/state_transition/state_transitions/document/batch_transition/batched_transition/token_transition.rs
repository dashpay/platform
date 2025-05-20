use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use platform_value::Identifier;
use bincode::{Encode, Decode};
use platform_version::version::PlatformVersion;
use crate::balances::credits::TokenAmount;
use crate::block::block_info::BlockInfo;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_distribution_key::{TokenDistributionType, TokenDistributionTypeWithResolvedRecipient};
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::{TokenDistributionRecipient, TokenDistributionResolvedRecipient};
use crate::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use crate::data_contract::DataContract;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use crate::state_transition::state_transitions::document::batch_transition::{DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition, TokenBurnTransition, TokenConfigUpdateTransition, TokenDestroyFrozenFundsTransition, TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition, TokenClaimTransition, TokenTransferTransition, TokenSetPriceForDirectPurchaseTransition};
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::{DocumentPurchaseTransition, DocumentTransferTransition};
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_unfreeze_transition::TokenUnfreezeTransition;
use crate::state_transition::state_transitions::document::batch_transition::resolvers::v0::BatchTransitionResolversV0;
use crate::state_transition::state_transitions::document::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::state_transitions::document::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_direct_purchase_transition::TokenDirectPurchaseTransition;
use crate::state_transition::state_transitions::document::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use crate::tokens::token_event::TokenEvent;

pub const TOKEN_HISTORY_ID_BYTES: [u8; 32] = [
    45, 67, 89, 21, 34, 216, 145, 78, 156, 243, 17, 58, 202, 190, 13, 92, 61, 40, 122, 201, 84, 99,
    187, 110, 233, 128, 63, 48, 172, 29, 210, 108,
];

#[derive(Debug, Clone, Encode, Decode, From, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub enum TokenTransition {
    #[display("TokenBurnTransition({})", "_0")]
    Burn(TokenBurnTransition),

    #[display("TokenIssuanceTransition({})", "_0")]
    Mint(TokenMintTransition),

    #[display("TokenTransferTransition({})", "_0")]
    Transfer(TokenTransferTransition),

    #[display("TokenFreezeTransition({})", "_0")]
    Freeze(TokenFreezeTransition),

    #[display("TokenUnfreezeTransition({})", "_0")]
    Unfreeze(TokenUnfreezeTransition),

    #[display("TokenDestroyFrozenFundsTransition({})", "_0")]
    DestroyFrozenFunds(TokenDestroyFrozenFundsTransition),

    #[display("TokenClaimTransition({})", "_0")]
    Claim(TokenClaimTransition),

    #[display("TokenEmergencyActionTransition({})", "_0")]
    EmergencyAction(TokenEmergencyActionTransition),

    #[display("TokenConfigUpdateTransition({})", "_0")]
    ConfigUpdate(TokenConfigUpdateTransition),

    #[display("TokenDirectPurchaseTransition({})", "_0")]
    DirectPurchase(TokenDirectPurchaseTransition),

    #[display("TokenSetPriceForDirectPurchaseTransition({})", "_0")]
    SetPriceForDirectPurchase(TokenSetPriceForDirectPurchaseTransition),
}

impl BatchTransitionResolversV0 for TokenTransition {
    fn as_transition_create(&self) -> Option<&DocumentCreateTransition> {
        None
    }
    fn as_transition_replace(&self) -> Option<&DocumentReplaceTransition> {
        None
    }

    fn as_transition_delete(&self) -> Option<&DocumentDeleteTransition> {
        None
    }

    fn as_transition_transfer(&self) -> Option<&DocumentTransferTransition> {
        None
    }

    fn as_transition_purchase(&self) -> Option<&DocumentPurchaseTransition> {
        None
    }

    fn as_transition_token_burn(&self) -> Option<&TokenBurnTransition> {
        if let Self::Burn(ref t) = self {
            Some(t)
        } else {
            None
        }
    }
    fn as_transition_token_mint(&self) -> Option<&TokenMintTransition> {
        if let Self::Mint(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_transfer(&self) -> Option<&TokenTransferTransition> {
        if let Self::Transfer(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_freeze(&self) -> Option<&TokenFreezeTransition> {
        if let Self::Freeze(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_unfreeze(&self) -> Option<&TokenUnfreezeTransition> {
        if let Self::Unfreeze(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_destroy_frozen_funds(
        &self,
    ) -> Option<&TokenDestroyFrozenFundsTransition> {
        if let Self::DestroyFrozenFunds(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_claim(&self) -> Option<&TokenClaimTransition> {
        if let Self::Claim(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_emergency_action(&self) -> Option<&TokenEmergencyActionTransition> {
        if let Self::EmergencyAction(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_config_update(&self) -> Option<&TokenConfigUpdateTransition> {
        if let Self::ConfigUpdate(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_direct_purchase(&self) -> Option<&TokenDirectPurchaseTransition> {
        if let Self::DirectPurchase(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_set_price_for_direct_purchase(
        &self,
    ) -> Option<&TokenSetPriceForDirectPurchaseTransition> {
        if let Self::SetPriceForDirectPurchase(ref t) = self {
            Some(t)
        } else {
            None
        }
    }
}

pub trait TokenTransitionV0Methods {
    fn base(&self) -> &TokenBaseTransition;
    fn base_mut(&mut self) -> &mut TokenBaseTransition;
    /// get the data contract ID
    fn data_contract_id(&self) -> Identifier;
    /// set data contract's ID
    fn set_data_contract_id(&mut self, id: Identifier);

    /// get the token ID
    fn token_id(&self) -> Identifier;

    /// set the token ID
    fn set_token_id(&mut self, id: Identifier);

    /// get the identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce;
    /// sets identity contract nonce
    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce);

    fn calculate_action_id(&self, owner_id: Identifier) -> Option<Identifier>;

    fn can_calculate_action_id(&self) -> bool;
    /// Historical document type name for the token history contract
    fn historical_document_type_name(&self) -> &str;
    /// Historical document type for the token history contract
    fn historical_document_type<'a>(
        &self,
        token_history_contract: &'a DataContract,
    ) -> Result<DocumentTypeRef<'a>, ProtocolError>;
    /// Historical document id
    fn historical_document_id(&self, owner_id: Identifier) -> Identifier;
    fn associated_token_event(
        &self,
        token_configuration: &TokenConfiguration,
        contract_owner_id: Identifier,
    ) -> Result<TokenEvent, ProtocolError>;
    /// Historical document id
    fn build_historical_document(
        &self,
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        block_info: &BlockInfo,
        token_configuration: &TokenConfiguration,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;
}

impl TokenTransitionV0Methods for TokenTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenTransition::Burn(t) => t.base(),
            TokenTransition::Mint(t) => t.base(),
            TokenTransition::Transfer(t) => t.base(),
            TokenTransition::Freeze(t) => t.base(),
            TokenTransition::Unfreeze(t) => t.base(),
            TokenTransition::DestroyFrozenFunds(t) => t.base(),
            TokenTransition::Claim(t) => t.base(),
            TokenTransition::EmergencyAction(t) => t.base(),
            TokenTransition::ConfigUpdate(t) => t.base(),
            TokenTransition::DirectPurchase(t) => t.base(),
            TokenTransition::SetPriceForDirectPurchase(t) => t.base(),
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenTransition::Burn(t) => t.base_mut(),
            TokenTransition::Mint(t) => t.base_mut(),
            TokenTransition::Transfer(t) => t.base_mut(),
            TokenTransition::Freeze(t) => t.base_mut(),
            TokenTransition::Unfreeze(t) => t.base_mut(),
            TokenTransition::DestroyFrozenFunds(t) => t.base_mut(),
            TokenTransition::Claim(t) => t.base_mut(),
            TokenTransition::EmergencyAction(t) => t.base_mut(),
            TokenTransition::ConfigUpdate(t) => t.base_mut(),
            TokenTransition::DirectPurchase(t) => t.base_mut(),
            TokenTransition::SetPriceForDirectPurchase(t) => t.base_mut(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    fn calculate_action_id(&self, owner_id: Identifier) -> Option<Identifier> {
        match self {
            TokenTransition::Burn(t) => Some(t.calculate_action_id(owner_id)),
            TokenTransition::Mint(t) => Some(t.calculate_action_id(owner_id)),
            TokenTransition::Freeze(t) => Some(t.calculate_action_id(owner_id)),
            TokenTransition::Unfreeze(t) => Some(t.calculate_action_id(owner_id)),
            TokenTransition::Transfer(_) => None,
            TokenTransition::DestroyFrozenFunds(t) => Some(t.calculate_action_id(owner_id)),
            TokenTransition::Claim(_) => None,
            TokenTransition::EmergencyAction(t) => Some(t.calculate_action_id(owner_id)),
            TokenTransition::ConfigUpdate(t) => Some(t.calculate_action_id(owner_id)),
            TokenTransition::DirectPurchase(_) => None,
            TokenTransition::SetPriceForDirectPurchase(t) => Some(t.calculate_action_id(owner_id)),
        }
    }

    fn can_calculate_action_id(&self) -> bool {
        match self {
            TokenTransition::Burn(_)
            | TokenTransition::Mint(_)
            | TokenTransition::Freeze(_)
            | TokenTransition::Unfreeze(_)
            | TokenTransition::DestroyFrozenFunds(_)
            | TokenTransition::EmergencyAction(_)
            | TokenTransition::ConfigUpdate(_)
            | TokenTransition::SetPriceForDirectPurchase(_) => true,
            TokenTransition::Transfer(_)
            | TokenTransition::Claim(_)
            | TokenTransition::DirectPurchase(_) => false,
        }
    }

    fn set_data_contract_id(&mut self, id: Identifier) {
        self.base_mut().set_data_contract_id(id);
    }

    fn token_id(&self) -> Identifier {
        self.base().token_id()
    }

    fn set_token_id(&mut self, token_id: Identifier) {
        self.base_mut().set_token_id(token_id)
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.base().identity_contract_nonce()
    }

    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        self.base_mut().set_identity_contract_nonce(nonce);
    }

    /// Historical document type name for the token history contract
    fn historical_document_type_name(&self) -> &str {
        match self {
            TokenTransition::Burn(_) => "burn",
            TokenTransition::Mint(_) => "mint",
            TokenTransition::Transfer(_) => "transfer",
            TokenTransition::Freeze(_) => "freeze",
            TokenTransition::Unfreeze(_) => "unfreeze",
            TokenTransition::EmergencyAction(_) => "emergencyAction",
            TokenTransition::DestroyFrozenFunds(_) => "destroyFrozenFunds",
            TokenTransition::ConfigUpdate(_) => "configUpdate",
            TokenTransition::Claim(_) => "claim",
            TokenTransition::DirectPurchase(_) => "directPurchase",
            TokenTransition::SetPriceForDirectPurchase(_) => "directPricing",
        }
    }

    /// Historical document type for the token history contract
    fn historical_document_type<'a>(
        &self,
        token_history_contract: &'a DataContract,
    ) -> Result<DocumentTypeRef<'a>, ProtocolError> {
        Ok(token_history_contract.document_type_for_name(self.historical_document_type_name())?)
    }

    /// Historical document id
    fn historical_document_id(&self, owner_id: Identifier) -> Identifier {
        let token_id = self.token_id();
        let name = self.historical_document_type_name();
        let owner_nonce = self.identity_contract_nonce();
        Document::generate_document_id_v0(
            &token_id,
            &owner_id,
            format!("history_{}", name).as_str(),
            owner_nonce.to_be_bytes().as_slice(),
        )
    }

    /// Historical document id
    fn build_historical_document(
        &self,
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        block_info: &BlockInfo,
        token_configuration: &TokenConfiguration,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        self.associated_token_event(token_configuration, owner_id)?
            .build_historical_document_owned(
                token_id,
                owner_id,
                owner_nonce,
                block_info,
                platform_version,
            )
    }

    fn associated_token_event(
        &self,
        token_configuration: &TokenConfiguration,
        owner_id: Identifier,
    ) -> Result<TokenEvent, ProtocolError> {
        Ok(match self {
            TokenTransition::Burn(burn) => {
                // The owner id might be incorrect when doing group actions
                // However it will be fixed in verify_state_transition was executed area.
                TokenEvent::Burn(burn.burn_amount(), owner_id, burn.public_note().cloned())
            }
            TokenTransition::Mint(mint) => {
                let recipient = match mint.issued_to_identity_id() {
                    None => token_configuration.distribution_rules().new_tokens_destination_identity().copied().ok_or(ProtocolError::NotSupported("either the mint destination must be set or the contract must have a destination set for new tokens".to_string()))?,
                    Some(recipient) => recipient,
                };
                TokenEvent::Mint(mint.amount(), recipient, mint.public_note().cloned())
            }
            TokenTransition::Transfer(transfer) => {
                let (public_note, shared_encrypted_note, private_encrypted_note) = transfer.notes();
                TokenEvent::Transfer(
                    transfer.recipient_id(),
                    public_note,
                    shared_encrypted_note,
                    private_encrypted_note,
                    transfer.amount(),
                )
            }
            TokenTransition::Freeze(freeze) => {
                TokenEvent::Freeze(freeze.frozen_identity_id(), freeze.public_note().cloned())
            }
            TokenTransition::Unfreeze(unfreeze) => TokenEvent::Unfreeze(
                unfreeze.frozen_identity_id(),
                unfreeze.public_note().cloned(),
            ),
            TokenTransition::EmergencyAction(emergency_action) => TokenEvent::EmergencyAction(
                emergency_action.emergency_action(),
                emergency_action.public_note().cloned(),
            ),
            TokenTransition::DestroyFrozenFunds(destroy_frozen_funds) => {
                TokenEvent::DestroyFrozenFunds(
                    destroy_frozen_funds.frozen_identity_id(),
                    TokenAmount::MAX, // we do not know how much will be destroyed
                    destroy_frozen_funds.public_note().cloned(),
                )
            }
            TokenTransition::ConfigUpdate(config_update) => TokenEvent::ConfigUpdate(
                config_update.update_token_configuration_item().clone(),
                config_update.public_note().cloned(),
            ),
            TokenTransition::Claim(claim) => {
                let distribution_rules = token_configuration.distribution_rules();
                let distribution_recipient = match claim.distribution_type() {
                    TokenDistributionType::PreProgrammed => {
                        if distribution_rules.pre_programmed_distribution().is_none() {
                            return Err(ProtocolError::NotSupported("Token claiming of pre programmed distribution is not supported on this token".to_string()));
                        }
                        TokenDistributionTypeWithResolvedRecipient::PreProgrammed(owner_id)
                    }
                    TokenDistributionType::Perpetual => {
                        let Some(perpetual_distribution) =
                            distribution_rules.perpetual_distribution()
                        else {
                            return Err(ProtocolError::NotSupported("Token claiming of perpetual distribution is not supported on this token".to_string()));
                        };
                        let recipient = match perpetual_distribution.distribution_recipient() {
                            TokenDistributionRecipient::ContractOwner => {
                                TokenDistributionResolvedRecipient::ContractOwnerIdentity(owner_id)
                            }
                            TokenDistributionRecipient::Identity(identifier) => {
                                TokenDistributionResolvedRecipient::Identity(identifier)
                            }
                            TokenDistributionRecipient::EvonodesByParticipation => {
                                TokenDistributionResolvedRecipient::Evonode(owner_id)
                            }
                        };
                        TokenDistributionTypeWithResolvedRecipient::Perpetual(recipient)
                    }
                };

                TokenEvent::Claim(
                    distribution_recipient,
                    TokenAmount::MAX, // we do not know how much will be released
                    claim.public_note().cloned(),
                )
            }
            TokenTransition::DirectPurchase(direct_purchase) => TokenEvent::DirectPurchase(
                direct_purchase.token_count(),
                direct_purchase.total_agreed_price(),
            ),
            TokenTransition::SetPriceForDirectPurchase(set_price_transition) => {
                TokenEvent::ChangePriceForDirectPurchase(
                    set_price_transition.price().cloned(),
                    set_price_transition.public_note().cloned(),
                )
            }
        })
    }
}
