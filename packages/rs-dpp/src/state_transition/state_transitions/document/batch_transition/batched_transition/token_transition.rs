use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
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
use crate::state_transition::batch_transition::{DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition, TokenBurnTransition, TokenConfigUpdateTransition, TokenDestroyFrozenFundsTransition, TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition, TokenClaimTransition, TokenTransferTransition, TokenSetPriceForDirectPurchaseTransition};
use crate::state_transition::batch_transition::batched_transition::{DocumentPurchaseTransition, DocumentTransferTransition};
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::batched_transition::token_unfreeze_transition::TokenUnfreezeTransition;
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use crate::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use crate::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use crate::state_transition::batch_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use crate::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use crate::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use crate::state_transition::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use crate::state_transition::batch_transition::token_direct_purchase_transition::TokenDirectPurchaseTransition;
use crate::state_transition::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use crate::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use crate::tokens::token_event::TokenEvent;

pub const TOKEN_HISTORY_ID_BYTES: [u8; 32] = [
    45, 67, 89, 21, 34, 216, 145, 78, 156, 243, 17, 58, 202, 190, 13, 92, 61, 40, 122, 201, 84, 99,
    187, 110, 233, 128, 63, 48, 172, 29, 210, 108,
];

#[derive(Debug, Clone, Encode, Decode, From, PartialEq, Display)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
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

    fn calculate_action_id(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Option<Result<Identifier, ProtocolError>>;

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

    fn calculate_action_id(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Option<Result<Identifier, ProtocolError>> {
        match self {
            TokenTransition::Burn(t) => Some(t.calculate_action_id(owner_id, platform_version)),
            TokenTransition::Mint(t) => Some(t.calculate_action_id(owner_id, platform_version)),
            TokenTransition::Freeze(t) => Some(t.calculate_action_id(owner_id, platform_version)),
            TokenTransition::Unfreeze(t) => Some(t.calculate_action_id(owner_id, platform_version)),
            TokenTransition::Transfer(_) => None,
            TokenTransition::DestroyFrozenFunds(t) => {
                Some(t.calculate_action_id(owner_id, platform_version))
            }
            TokenTransition::Claim(_) => None,
            TokenTransition::EmergencyAction(t) => {
                Some(t.calculate_action_id(owner_id, platform_version))
            }
            TokenTransition::ConfigUpdate(t) => {
                Some(t.calculate_action_id(owner_id, platform_version))
            }
            TokenTransition::DirectPurchase(_) => None,
            TokenTransition::SetPriceForDirectPurchase(t) => {
                Some(t.calculate_action_id(owner_id, platform_version))
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
    use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
    use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
    use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
    use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use crate::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use crate::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;
    use crate::state_transition::batch_transition::batched_transition::token_burn_transition::TokenBurnTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_claim_transition::TokenClaimTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_config_update_transition::TokenConfigUpdateTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_direct_purchase_transition::TokenDirectPurchaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_emergency_action_transition::TokenEmergencyActionTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_freeze_transition::TokenFreezeTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_mint_transition::TokenMintTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::TokenTransferTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_unfreeze_transition::TokenUnfreezeTransitionV0;
    use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use crate::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
    use crate::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
    use crate::data_contract::associated_token::token_marketplace_rules::v0::{
        TokenMarketplaceRulesV0, TokenTradeMode,
    };
    use crate::data_contract::associated_token::token_marketplace_rules::TokenMarketplaceRules;
    use crate::tokens::emergency_action::TokenEmergencyAction;
    use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
    use std::collections::BTreeMap;

    /// Helper: build a default ChangeControlRules (NoOne / NoOne).
    fn no_one_rules() -> ChangeControlRules {
        ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::NoOne,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        })
    }

    /// Helper: build a TokenConfiguration whose distribution_rules have
    /// `new_tokens_destination_identity` set to the given identifier (or None).
    fn make_token_config(
        new_tokens_dest: Option<Identifier>,
        pre_programmed: Option<TokenPreProgrammedDistribution>,
        perpetual: Option<TokenPerpetualDistribution>,
    ) -> TokenConfiguration {
        TokenConfiguration::V0(TokenConfigurationV0 {
            conventions: TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
                localizations: BTreeMap::new(),
                decimals: 8,
            }),
            conventions_change_rules: no_one_rules(),
            base_supply: 1000,
            max_supply: None,
            keeps_history: TokenKeepsHistoryRules::V0(TokenKeepsHistoryRulesV0 {
                keeps_transfer_history: true,
                keeps_freezing_history: true,
                keeps_minting_history: true,
                keeps_burning_history: true,
                keeps_direct_pricing_history: true,
                keeps_direct_purchase_history: true,
            }),
            start_as_paused: false,
            allow_transfer_to_frozen_balance: true,
            max_supply_change_rules: no_one_rules(),
            distribution_rules: TokenDistributionRules::V0(TokenDistributionRulesV0 {
                perpetual_distribution: perpetual,
                perpetual_distribution_rules: no_one_rules(),
                pre_programmed_distribution: pre_programmed,
                new_tokens_destination_identity: new_tokens_dest,
                new_tokens_destination_identity_rules: no_one_rules(),
                minting_allow_choosing_destination: true,
                minting_allow_choosing_destination_rules: no_one_rules(),
                change_direct_purchase_pricing_rules: no_one_rules(),
            }),
            marketplace_rules: TokenMarketplaceRules::V0(TokenMarketplaceRulesV0 {
                trade_mode: TokenTradeMode::NotTradeable,
                trade_mode_change_rules: no_one_rules(),
            }),
            manual_minting_rules: no_one_rules(),
            manual_burning_rules: no_one_rules(),
            freeze_rules: no_one_rules(),
            unfreeze_rules: no_one_rules(),
            destroy_frozen_funds_rules: no_one_rules(),
            emergency_action_rules: no_one_rules(),
            main_control_group: None,
            main_control_group_can_be_modified: AuthorizedActionTakers::NoOne,
            description: None,
        })
    }

    // -----------------------------------------------------------------------
    // historical_document_type_name
    // -----------------------------------------------------------------------

    #[test]
    fn historical_document_type_name_returns_correct_string_per_variant() {
        let base = TokenBaseTransition::default();
        let cases: Vec<(TokenTransition, &str)> = vec![
            (
                TokenTransition::Burn(TokenBurnTransition::V0(TokenBurnTransitionV0 {
                    base: base.clone(),
                    burn_amount: 0,
                    public_note: None,
                })),
                "burn",
            ),
            (
                TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
                    base: base.clone(),
                    issued_to_identity_id: None,
                    amount: 0,
                    public_note: None,
                })),
                "mint",
            ),
            (
                TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                    base: base.clone(),
                    amount: 0,
                    recipient_id: Identifier::default(),
                    public_note: None,
                    shared_encrypted_note: None,
                    private_encrypted_note: None,
                })),
                "transfer",
            ),
            (
                TokenTransition::Freeze(TokenFreezeTransition::V0(TokenFreezeTransitionV0 {
                    base: base.clone(),
                    identity_to_freeze_id: Identifier::default(),
                    public_note: None,
                })),
                "freeze",
            ),
            (
                TokenTransition::Unfreeze(TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0 {
                    base: base.clone(),
                    frozen_identity_id: Identifier::default(),
                    public_note: None,
                })),
                "unfreeze",
            ),
            (
                TokenTransition::EmergencyAction(TokenEmergencyActionTransition::V0(
                    TokenEmergencyActionTransitionV0 {
                        base: base.clone(),
                        emergency_action: TokenEmergencyAction::Pause,
                        public_note: None,
                    },
                )),
                "emergencyAction",
            ),
            (
                TokenTransition::DestroyFrozenFunds(TokenDestroyFrozenFundsTransition::V0(
                    TokenDestroyFrozenFundsTransitionV0 {
                        base: base.clone(),
                        frozen_identity_id: Identifier::default(),
                        public_note: None,
                    },
                )),
                "destroyFrozenFunds",
            ),
            (
                TokenTransition::ConfigUpdate(TokenConfigUpdateTransition::V0(
                    TokenConfigUpdateTransitionV0 {
                        base: base.clone(),
                        update_token_configuration_item:
                            TokenConfigurationChangeItem::TokenConfigurationNoChange,
                        public_note: None,
                    },
                )),
                "configUpdate",
            ),
            (
                TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
                    base: base.clone(),
                    distribution_type: TokenDistributionType::PreProgrammed,
                    public_note: None,
                })),
                "claim",
            ),
            (
                TokenTransition::DirectPurchase(TokenDirectPurchaseTransition::V0(
                    TokenDirectPurchaseTransitionV0 {
                        base: base.clone(),
                        token_count: 0,
                        total_agreed_price: 0,
                    },
                )),
                "directPurchase",
            ),
            (
                TokenTransition::SetPriceForDirectPurchase(
                    TokenSetPriceForDirectPurchaseTransition::V0(
                        TokenSetPriceForDirectPurchaseTransitionV0 {
                            base: base.clone(),
                            price: None,
                            public_note: None,
                        },
                    ),
                ),
                "directPricing",
            ),
        ];

        for (transition, expected_name) in cases {
            assert_eq!(
                transition.historical_document_type_name(),
                expected_name,
                "Mismatch for variant that should return '{}'",
                expected_name
            );
        }
    }

    // -----------------------------------------------------------------------
    // can_calculate_action_id
    // -----------------------------------------------------------------------

    #[test]
    fn can_calculate_action_id_returns_false_for_transfer_claim_direct_purchase() {
        let base = TokenBaseTransition::default();

        let transfer =
            TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                base: base.clone(),
                amount: 100,
                recipient_id: Identifier::default(),
                public_note: None,
                shared_encrypted_note: None,
                private_encrypted_note: None,
            }));
        assert!(!transfer.can_calculate_action_id());

        let claim = TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: base.clone(),
            distribution_type: TokenDistributionType::PreProgrammed,
            public_note: None,
        }));
        assert!(!claim.can_calculate_action_id());

        let direct_purchase = TokenTransition::DirectPurchase(TokenDirectPurchaseTransition::V0(
            TokenDirectPurchaseTransitionV0 {
                base: base.clone(),
                token_count: 1,
                total_agreed_price: 100,
            },
        ));
        assert!(!direct_purchase.can_calculate_action_id());
    }

    #[test]
    fn can_calculate_action_id_returns_true_for_all_other_variants() {
        let base = TokenBaseTransition::default();

        let variants: Vec<TokenTransition> = vec![
            TokenTransition::Burn(TokenBurnTransition::V0(TokenBurnTransitionV0 {
                base: base.clone(),
                burn_amount: 50,
                public_note: None,
            })),
            TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
                base: base.clone(),
                issued_to_identity_id: None,
                amount: 100,
                public_note: None,
            })),
            TokenTransition::Freeze(TokenFreezeTransition::V0(TokenFreezeTransitionV0 {
                base: base.clone(),
                identity_to_freeze_id: Identifier::default(),
                public_note: None,
            })),
            TokenTransition::Unfreeze(TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0 {
                base: base.clone(),
                frozen_identity_id: Identifier::default(),
                public_note: None,
            })),
            TokenTransition::DestroyFrozenFunds(TokenDestroyFrozenFundsTransition::V0(
                TokenDestroyFrozenFundsTransitionV0 {
                    base: base.clone(),
                    frozen_identity_id: Identifier::default(),
                    public_note: None,
                },
            )),
            TokenTransition::EmergencyAction(TokenEmergencyActionTransition::V0(
                TokenEmergencyActionTransitionV0 {
                    base: base.clone(),
                    emergency_action: TokenEmergencyAction::Pause,
                    public_note: None,
                },
            )),
            TokenTransition::ConfigUpdate(TokenConfigUpdateTransition::V0(
                TokenConfigUpdateTransitionV0 {
                    base: base.clone(),
                    update_token_configuration_item:
                        TokenConfigurationChangeItem::TokenConfigurationNoChange,
                    public_note: None,
                },
            )),
            TokenTransition::SetPriceForDirectPurchase(
                TokenSetPriceForDirectPurchaseTransition::V0(
                    TokenSetPriceForDirectPurchaseTransitionV0 {
                        base: base.clone(),
                        price: None,
                        public_note: None,
                    },
                ),
            ),
        ];

        for variant in variants {
            assert!(
                variant.can_calculate_action_id(),
                "Expected can_calculate_action_id() == true for {:?}",
                variant
            );
        }
    }

    // -----------------------------------------------------------------------
    // calculate_action_id
    // -----------------------------------------------------------------------

    #[test]
    fn calculate_action_id_returns_none_for_transfer_claim_direct_purchase() {
        let base = TokenBaseTransition::default();
        let owner_id = Identifier::from([1u8; 32]);

        let transfer =
            TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                base: base.clone(),
                amount: 100,
                recipient_id: Identifier::default(),
                public_note: None,
                shared_encrypted_note: None,
                private_encrypted_note: None,
            }));
        assert!(transfer.calculate_action_id(owner_id).is_none());

        let claim = TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: base.clone(),
            distribution_type: TokenDistributionType::PreProgrammed,
            public_note: None,
        }));
        assert!(claim.calculate_action_id(owner_id).is_none());

        let direct_purchase = TokenTransition::DirectPurchase(TokenDirectPurchaseTransition::V0(
            TokenDirectPurchaseTransitionV0 {
                base: base.clone(),
                token_count: 1,
                total_agreed_price: 100,
            },
        ));
        assert!(direct_purchase.calculate_action_id(owner_id).is_none());
    }

    #[test]
    fn calculate_action_id_returns_some_for_burn() {
        let base = TokenBaseTransition::default();
        let owner_id = Identifier::from([2u8; 32]);

        let burn = TokenTransition::Burn(TokenBurnTransition::V0(TokenBurnTransitionV0 {
            base: base.clone(),
            burn_amount: 50,
            public_note: None,
        }));
        let result = burn.calculate_action_id(owner_id);
        assert!(result.is_some());
    }

    #[test]
    fn calculate_action_id_returns_some_for_mint() {
        let base = TokenBaseTransition::default();
        let owner_id = Identifier::from([3u8; 32]);

        let mint = TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
            base: base.clone(),
            issued_to_identity_id: None,
            amount: 100,
            public_note: None,
        }));
        let result = mint.calculate_action_id(owner_id);
        assert!(result.is_some());
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Burn
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_burn_produces_burn_event() {
        let owner_id = Identifier::from([10u8; 32]);
        let config = make_token_config(None, None, None);

        let transition = TokenTransition::Burn(TokenBurnTransition::V0(TokenBurnTransitionV0 {
            base: TokenBaseTransition::default(),
            burn_amount: 500,
            public_note: Some("burn note".to_string()),
        }));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce Burn event");
        match event {
            TokenEvent::Burn(amount, burner, note) => {
                assert_eq!(amount, 500);
                assert_eq!(burner, owner_id);
                assert_eq!(note, Some("burn note".to_string()));
            }
            other => panic!("Expected Burn, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Mint (explicit recipient)
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_mint_with_explicit_recipient() {
        let owner_id = Identifier::from([10u8; 32]);
        let recipient = Identifier::from([20u8; 32]);
        let config = make_token_config(None, None, None);

        let transition = TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
            base: TokenBaseTransition::default(),
            issued_to_identity_id: Some(recipient),
            amount: 1000,
            public_note: None,
        }));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce Mint event");
        match event {
            TokenEvent::Mint(amount, recv, note) => {
                assert_eq!(amount, 1000);
                assert_eq!(recv, recipient);
                assert!(note.is_none());
            }
            other => panic!("Expected Mint, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Mint (fallback to config destination)
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_mint_falls_back_to_config_destination() {
        let owner_id = Identifier::from([10u8; 32]);
        let config_dest = Identifier::from([30u8; 32]);
        let config = make_token_config(Some(config_dest), None, None);

        let transition = TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
            base: TokenBaseTransition::default(),
            issued_to_identity_id: None,
            amount: 500,
            public_note: None,
        }));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should fall back to config destination");
        match event {
            TokenEvent::Mint(amount, recv, _) => {
                assert_eq!(amount, 500);
                assert_eq!(recv, config_dest);
            }
            other => panic!("Expected Mint, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Mint (no recipient anywhere -> error)
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_mint_errors_when_no_recipient_available() {
        let owner_id = Identifier::from([10u8; 32]);
        // Config with no new_tokens_destination_identity
        let config = make_token_config(None, None, None);

        let transition = TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
            base: TokenBaseTransition::default(),
            issued_to_identity_id: None,
            amount: 500,
            public_note: None,
        }));

        let result = transition.associated_token_event(&config, owner_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::NotSupported(msg) => {
                assert!(msg.contains("mint destination"));
            }
            other => panic!("Expected NotSupported, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Transfer
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_transfer_produces_transfer_event() {
        let owner_id = Identifier::from([10u8; 32]);
        let recipient = Identifier::from([20u8; 32]);
        let config = make_token_config(None, None, None);

        let transition =
            TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                base: TokenBaseTransition::default(),
                amount: 250,
                recipient_id: recipient,
                public_note: Some("transfer note".to_string()),
                shared_encrypted_note: None,
                private_encrypted_note: None,
            }));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce Transfer event");
        match event {
            TokenEvent::Transfer(recv, public_note, shared, private, amount) => {
                assert_eq!(recv, recipient);
                assert_eq!(amount, 250);
                assert_eq!(public_note, Some("transfer note".to_string()));
                assert!(shared.is_none());
                assert!(private.is_none());
            }
            other => panic!("Expected Transfer, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Freeze / Unfreeze
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_freeze_and_unfreeze() {
        let owner_id = Identifier::from([10u8; 32]);
        let frozen_id = Identifier::from([40u8; 32]);
        let config = make_token_config(None, None, None);

        let freeze = TokenTransition::Freeze(TokenFreezeTransition::V0(TokenFreezeTransitionV0 {
            base: TokenBaseTransition::default(),
            identity_to_freeze_id: frozen_id,
            public_note: Some("freeze!".to_string()),
        }));
        let event = freeze
            .associated_token_event(&config, owner_id)
            .expect("should produce Freeze event");
        match event {
            TokenEvent::Freeze(id, note) => {
                assert_eq!(id, frozen_id);
                assert_eq!(note, Some("freeze!".to_string()));
            }
            other => panic!("Expected Freeze, got {:?}", other),
        }

        let unfreeze =
            TokenTransition::Unfreeze(TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0 {
                base: TokenBaseTransition::default(),
                frozen_identity_id: frozen_id,
                public_note: None,
            }));
        let event = unfreeze
            .associated_token_event(&config, owner_id)
            .expect("should produce Unfreeze event");
        match event {
            TokenEvent::Unfreeze(id, note) => {
                assert_eq!(id, frozen_id);
                assert!(note.is_none());
            }
            other => panic!("Expected Unfreeze, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — EmergencyAction
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_emergency_action() {
        let owner_id = Identifier::from([10u8; 32]);
        let config = make_token_config(None, None, None);

        let transition = TokenTransition::EmergencyAction(TokenEmergencyActionTransition::V0(
            TokenEmergencyActionTransitionV0 {
                base: TokenBaseTransition::default(),
                emergency_action: TokenEmergencyAction::Pause,
                public_note: Some("pausing".to_string()),
            },
        ));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce EmergencyAction event");
        match event {
            TokenEvent::EmergencyAction(action, note) => {
                assert_eq!(action, TokenEmergencyAction::Pause);
                assert_eq!(note, Some("pausing".to_string()));
            }
            other => panic!("Expected EmergencyAction, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — DestroyFrozenFunds
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_destroy_frozen_funds() {
        let owner_id = Identifier::from([10u8; 32]);
        let frozen_id = Identifier::from([50u8; 32]);
        let config = make_token_config(None, None, None);

        let transition = TokenTransition::DestroyFrozenFunds(
            TokenDestroyFrozenFundsTransition::V0(TokenDestroyFrozenFundsTransitionV0 {
                base: TokenBaseTransition::default(),
                frozen_identity_id: frozen_id,
                public_note: None,
            }),
        );

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce DestroyFrozenFunds event");
        match event {
            TokenEvent::DestroyFrozenFunds(id, amount, note) => {
                assert_eq!(id, frozen_id);
                assert_eq!(amount, TokenAmount::MAX);
                assert!(note.is_none());
            }
            other => panic!("Expected DestroyFrozenFunds, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — ConfigUpdate
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_config_update() {
        let owner_id = Identifier::from([10u8; 32]);
        let config = make_token_config(None, None, None);

        let transition = TokenTransition::ConfigUpdate(TokenConfigUpdateTransition::V0(
            TokenConfigUpdateTransitionV0 {
                base: TokenBaseTransition::default(),
                update_token_configuration_item: TokenConfigurationChangeItem::MaxSupply(Some(
                    9999,
                )),
                public_note: Some("cap update".to_string()),
            },
        ));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce ConfigUpdate event");
        match event {
            TokenEvent::ConfigUpdate(item, note) => {
                assert_eq!(item, TokenConfigurationChangeItem::MaxSupply(Some(9999)));
                assert_eq!(note, Some("cap update".to_string()));
            }
            other => panic!("Expected ConfigUpdate, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — DirectPurchase
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_direct_purchase() {
        let owner_id = Identifier::from([10u8; 32]);
        let config = make_token_config(None, None, None);

        let transition = TokenTransition::DirectPurchase(TokenDirectPurchaseTransition::V0(
            TokenDirectPurchaseTransitionV0 {
                base: TokenBaseTransition::default(),
                token_count: 42,
                total_agreed_price: 84000,
            },
        ));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce DirectPurchase event");
        match event {
            TokenEvent::DirectPurchase(count, price) => {
                assert_eq!(count, 42);
                assert_eq!(price, 84000);
            }
            other => panic!("Expected DirectPurchase, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — SetPriceForDirectPurchase
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_set_price_for_direct_purchase() {
        let owner_id = Identifier::from([10u8; 32]);
        let config = make_token_config(None, None, None);
        let pricing = TokenPricingSchedule::SinglePrice(5000);

        let transition = TokenTransition::SetPriceForDirectPurchase(
            TokenSetPriceForDirectPurchaseTransition::V0(
                TokenSetPriceForDirectPurchaseTransitionV0 {
                    base: TokenBaseTransition::default(),
                    price: Some(pricing.clone()),
                    public_note: Some("pricing change".to_string()),
                },
            ),
        );

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce ChangePriceForDirectPurchase event");
        match event {
            TokenEvent::ChangePriceForDirectPurchase(p, note) => {
                assert_eq!(p, Some(pricing));
                assert_eq!(note, Some("pricing change".to_string()));
            }
            other => panic!("Expected ChangePriceForDirectPurchase, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Claim (PreProgrammed, no distribution -> error)
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_claim_pre_programmed_errors_when_none() {
        let owner_id = Identifier::from([10u8; 32]);
        let config = make_token_config(None, None, None);

        let transition = TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: TokenBaseTransition::default(),
            distribution_type: TokenDistributionType::PreProgrammed,
            public_note: None,
        }));

        let result = transition.associated_token_event(&config, owner_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::NotSupported(msg) => {
                assert!(msg.contains("pre programmed"));
            }
            other => panic!("Expected NotSupported, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Claim (PreProgrammed, distribution present)
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_claim_pre_programmed_succeeds_when_present() {
        let owner_id = Identifier::from([10u8; 32]);
        let pre_prog = TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 {
            distributions: BTreeMap::new(),
        });
        let config = make_token_config(None, Some(pre_prog), None);

        let transition = TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: TokenBaseTransition::default(),
            distribution_type: TokenDistributionType::PreProgrammed,
            public_note: Some("claiming".to_string()),
        }));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce Claim event");
        match event {
            TokenEvent::Claim(dist_type, amount, note) => {
                assert_eq!(
                    dist_type,
                    TokenDistributionTypeWithResolvedRecipient::PreProgrammed(owner_id)
                );
                assert_eq!(amount, TokenAmount::MAX);
                assert_eq!(note, Some("claiming".to_string()));
            }
            other => panic!("Expected Claim, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Claim (Perpetual, no distribution -> error)
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_claim_perpetual_errors_when_none() {
        let owner_id = Identifier::from([10u8; 32]);
        let config = make_token_config(None, None, None);

        let transition = TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: TokenBaseTransition::default(),
            distribution_type: TokenDistributionType::Perpetual,
            public_note: None,
        }));

        let result = transition.associated_token_event(&config, owner_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::NotSupported(msg) => {
                assert!(msg.contains("perpetual"));
            }
            other => panic!("Expected NotSupported, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Claim (Perpetual, ContractOwner recipient)
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_claim_perpetual_contract_owner_recipient() {
        let owner_id = Identifier::from([10u8; 32]);
        let perpetual = TokenPerpetualDistribution::V0(TokenPerpetualDistributionV0 {
            distribution_type: RewardDistributionType::BlockBasedDistribution {
                interval: 10,
                function: DistributionFunction::FixedAmount { amount: 100 },
            },
            distribution_recipient: TokenDistributionRecipient::ContractOwner,
        });
        let config = make_token_config(None, None, Some(perpetual));

        let transition = TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: TokenBaseTransition::default(),
            distribution_type: TokenDistributionType::Perpetual,
            public_note: None,
        }));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce Claim event");
        match event {
            TokenEvent::Claim(dist_type, _, _) => {
                assert_eq!(
                    dist_type,
                    TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(owner_id)
                    )
                );
            }
            other => panic!("Expected Claim, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Claim (Perpetual, Identity recipient)
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_claim_perpetual_identity_recipient() {
        let owner_id = Identifier::from([10u8; 32]);
        let specific_id = Identifier::from([77u8; 32]);
        let perpetual = TokenPerpetualDistribution::V0(TokenPerpetualDistributionV0 {
            distribution_type: RewardDistributionType::BlockBasedDistribution {
                interval: 10,
                function: DistributionFunction::FixedAmount { amount: 100 },
            },
            distribution_recipient: TokenDistributionRecipient::Identity(specific_id),
        });
        let config = make_token_config(None, None, Some(perpetual));

        let transition = TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: TokenBaseTransition::default(),
            distribution_type: TokenDistributionType::Perpetual,
            public_note: None,
        }));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce Claim event");
        match event {
            TokenEvent::Claim(dist_type, _, _) => {
                assert_eq!(
                    dist_type,
                    TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Identity(specific_id)
                    )
                );
            }
            other => panic!("Expected Claim, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // associated_token_event — Claim (Perpetual, EvonodesByParticipation)
    // -----------------------------------------------------------------------

    #[test]
    fn associated_token_event_claim_perpetual_evonodes_recipient() {
        let owner_id = Identifier::from([10u8; 32]);
        let perpetual = TokenPerpetualDistribution::V0(TokenPerpetualDistributionV0 {
            distribution_type: RewardDistributionType::BlockBasedDistribution {
                interval: 10,
                function: DistributionFunction::FixedAmount { amount: 100 },
            },
            distribution_recipient: TokenDistributionRecipient::EvonodesByParticipation,
        });
        let config = make_token_config(None, None, Some(perpetual));

        let transition = TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: TokenBaseTransition::default(),
            distribution_type: TokenDistributionType::Perpetual,
            public_note: None,
        }));

        let event = transition
            .associated_token_event(&config, owner_id)
            .expect("should produce Claim event");
        match event {
            TokenEvent::Claim(dist_type, _, _) => {
                assert_eq!(
                    dist_type,
                    TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Evonode(owner_id)
                    )
                );
            }
            other => panic!("Expected Claim, got {:?}", other),
        }
    }
}
