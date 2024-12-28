use derive_more::From;
use std::sync::Arc;

use crate::drive::contract::DataContractFetchInfo;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;

/// transformer module for token burn transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenBurnTransitionActionV0)

use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};

/// Token burn transition action
#[derive(Debug, Clone, From)]
pub enum TokenBurnTransitionAction {
    /// v0
    V0(TokenBurnTransitionActionV0),
}

impl TokenBurnTransitionActionAccessorsV0 for TokenBurnTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenBurnTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.base,
        }
    }

    fn burn_amount(&self) -> u64 {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.burn_amount,
        }
    }

    fn set_burn_amount(&mut self, amount: u64) {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.burn_amount = amount,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}
