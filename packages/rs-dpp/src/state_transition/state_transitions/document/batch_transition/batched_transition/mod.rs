use bincode::{Decode, Encode};
use derive_more::From;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod document_base_transition;
pub mod document_create_transition;
pub mod document_delete_transition;
pub mod document_purchase_transition;
pub mod document_replace_transition;
pub mod document_transfer_transition;
pub mod document_transition;
pub mod document_transition_action_type;
pub mod document_update_price_transition;
pub mod multi_party_action;
mod resolvers;
pub mod token_base_transition;
pub mod token_burn_transition;
pub mod token_claim_transition;
pub mod token_config_update_transition;
pub mod token_destroy_frozen_funds_transition;
pub mod token_direct_purchase_transition;
pub mod token_emergency_action_transition;
pub mod token_freeze_transition;
pub mod token_mint_transition;
pub mod token_set_price_for_direct_purchase_transition;
pub mod token_transfer_transition;
pub mod token_transition;
pub mod token_transition_action_type;
pub mod token_unfreeze_transition;

use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use derive_more::Display;
pub use document_create_transition::DocumentCreateTransition;
pub use document_delete_transition::DocumentDeleteTransition;
pub use document_purchase_transition::DocumentPurchaseTransition;
pub use document_replace_transition::DocumentReplaceTransition;
pub use document_transfer_transition::DocumentTransferTransition;
use document_transition::DocumentTransition;
pub use document_update_price_transition::DocumentUpdatePriceTransition;
use platform_value::Identifier;
use token_transition::TokenTransition;

pub const PROPERTY_ACTION: &str = "$action";

#[derive(Debug, Clone, Encode, Decode, From, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum BatchedTransition {
    #[display("DocumentTransition({})", "_0")]
    Document(DocumentTransition),
    #[display("TokenTransition({})", "_0")]
    Token(TokenTransition),
}

#[derive(Debug, From, Clone, Copy, PartialEq, Display)]
pub enum BatchedTransitionRef<'a> {
    #[display("DocumentTransition({})", "_0")]
    Document(&'a DocumentTransition),
    #[display("TokenTransition({})", "_0")]
    Token(&'a TokenTransition),
}

#[derive(Debug, From, PartialEq, Display)]
pub enum BatchedTransitionMutRef<'a> {
    #[display("DocumentTransition({})", "_0")]
    Document(&'a mut DocumentTransition),
    #[display("TokenTransition({})", "_0")]
    Token(&'a mut TokenTransition),
}

impl BatchedTransitionRef<'_> {
    pub fn to_owned_transition(&self) -> BatchedTransition {
        match self {
            BatchedTransitionRef::Document(doc_ref) => {
                BatchedTransition::Document((*doc_ref).clone())
            }
            BatchedTransitionRef::Token(tok_ref) => BatchedTransition::Token((*tok_ref).clone()),
        }
    }

    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            BatchedTransitionRef::Document(document_transition) => {
                document_transition.identity_contract_nonce()
            }
            BatchedTransitionRef::Token(token_transition) => {
                token_transition.identity_contract_nonce()
            }
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        match self {
            BatchedTransitionRef::Document(document_transition) => {
                document_transition.data_contract_id()
            }
            BatchedTransitionRef::Token(token_transition) => token_transition.data_contract_id(),
        }
    }
}

impl BatchedTransition {
    pub fn borrow_as_ref(&self) -> BatchedTransitionRef<'_> {
        match self {
            BatchedTransition::Document(doc) => {
                // Create a reference to a DocumentTransition
                BatchedTransitionRef::Document(doc)
            }
            BatchedTransition::Token(tok) => {
                // Create a reference to a TokenTransition
                BatchedTransitionRef::Token(tok)
            }
        }
    }

    pub fn borrow_as_mut(&mut self) -> BatchedTransitionMutRef<'_> {
        match self {
            BatchedTransition::Document(doc) => {
                // Create a reference to a DocumentTransition
                BatchedTransitionMutRef::Document(doc)
            }
            BatchedTransition::Token(tok) => {
                // Create a reference to a TokenTransition
                BatchedTransitionMutRef::Token(tok)
            }
        }
    }

    pub fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        match self {
            BatchedTransition::Document(document_transition) => {
                document_transition.set_identity_contract_nonce(identity_contract_nonce)
            }
            BatchedTransition::Token(token_transition) => {
                token_transition.set_identity_contract_nonce(identity_contract_nonce)
            }
        }
    }
}
