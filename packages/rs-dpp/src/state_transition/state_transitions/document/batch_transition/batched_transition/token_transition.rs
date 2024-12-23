use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use platform_value::Identifier;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::{DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition, TokenBurnTransition, TokenIssuanceTransition, TokenTransferTransition};
use crate::state_transition::batch_transition::batched_transition::{DocumentPurchaseTransition, DocumentTransferTransition};
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
    Issuance(TokenIssuanceTransition),

    #[display("TokenTransferTransition({})", "_0")]
    Transfer(TokenTransferTransition),
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
    fn as_transition_token_issuance(&self) -> Option<&TokenIssuanceTransition> {
        if let Self::Issuance(ref t) = self {
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
}

impl TokenTransitionV0Methods for TokenTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenTransition::Burn(t) => t.base(),
            TokenTransition::Issuance(t) => t.base(),
            TokenTransition::Transfer(t) => t.base(),
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenTransition::Burn(t) => t.base_mut(),
            TokenTransition::Issuance(t) => t.base_mut(),
            TokenTransition::Transfer(t) => t.base_mut(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
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
