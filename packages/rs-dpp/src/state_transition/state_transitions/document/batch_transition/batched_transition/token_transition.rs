use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use platform_value::Identifier;
use bincode::{Encode, Decode};
use data_contracts::SystemDataContract;
use platform_version::version::PlatformVersion;
use crate::balances::credits::TokenAmount;
use crate::block::block_info::BlockInfo;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::DataContract;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use crate::state_transition::batch_transition::{DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition, TokenBurnTransition, TokenConfigUpdateTransition, TokenDestroyFrozenFundsTransition, TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition, TokenTransferTransition};
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
use crate::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use crate::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use crate::tokens::token_event::TokenEvent;

#[derive(Debug, Clone, Encode, Decode, From, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
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

    #[display("TokenEmergencyActionTransition({})", "_0")]
    EmergencyAction(TokenEmergencyActionTransition),

    #[display("TokenConfigUpdateTransition({})", "_0")]
    ConfigUpdate(TokenConfigUpdateTransition),
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

    fn as_transition_token_emergency_action(&self) -> Option<&TokenEmergencyActionTransition> {
        if let Self::EmergencyAction(ref t) = self {
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
    fn historical_document_id(
        &self,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
    ) -> Identifier;
    fn associated_token_event(
        &self,
        token_configuration: &TokenConfiguration,
    ) -> Result<TokenEvent, ProtocolError>;
    /// Historical document id
    fn build_historical_document(
        &self,
        token_historical_contract: &DataContract,
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
            TokenTransition::EmergencyAction(t) => t.base(),
            TokenTransition::ConfigUpdate(t) => t.base(),
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
            TokenTransition::EmergencyAction(t) => t.base_mut(),
            TokenTransition::ConfigUpdate(t) => t.base_mut(),
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
            TokenTransition::EmergencyAction(t) => Some(t.calculate_action_id(owner_id)),
            TokenTransition::ConfigUpdate(t) => Some(t.calculate_action_id(owner_id)),
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
            | TokenTransition::ConfigUpdate(_) => true,
            TokenTransition::Transfer(_) => false,
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
    fn historical_document_id(
        &self,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
    ) -> Identifier {
        let name = self.historical_document_type_name();
        Document::generate_document_id_v0(
            &SystemDataContract::TokenHistory.id(),
            &owner_id,
            name,
            owner_nonce.to_be_bytes().as_slice(),
        )
    }

    /// Historical document id
    fn build_historical_document(
        &self,
        token_historical_contract: &DataContract,
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        block_info: &BlockInfo,
        token_configuration: &TokenConfiguration,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        self.associated_token_event(token_configuration)?
            .build_historical_document_owned(
                token_historical_contract,
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
    ) -> Result<TokenEvent, ProtocolError> {
        Ok(match self {
            TokenTransition::Burn(burn) => {
                TokenEvent::Burn(burn.burn_amount(), burn.public_note().cloned())
            }
            TokenTransition::Mint(mint) => {
                let recipient = match mint.issued_to_identity_id() {
                    None => token_configuration.new_tokens_destination_identity().ok_or(ProtocolError::NotSupported("either the mint destination must be set or the contract must have a destination set for new tokens".to_string()))?,
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
        })
    }
}
