use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use platform_value::Identifier;
use bincode::{Encode, Decode};
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::{DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition, TokenBurnTransition, TokenDestroyFrozenFundsTransition, TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition, TokenTransferTransition};
use crate::state_transition::batch_transition::batched_transition::{DocumentPurchaseTransition, DocumentTransferTransition};
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::batched_transition::token_unfreeze_transition::TokenUnfreezeTransition;
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;

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
        }
    }

    fn can_calculate_action_id(&self) -> bool {
        match self {
            TokenTransition::Burn(_)
            | TokenTransition::Mint(_)
            | TokenTransition::Freeze(_)
            | TokenTransition::Unfreeze(_)
            | TokenTransition::DestroyFrozenFunds(_)
            | TokenTransition::EmergencyAction(_) => true,
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
}
