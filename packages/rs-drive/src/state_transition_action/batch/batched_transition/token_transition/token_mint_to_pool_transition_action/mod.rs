use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::shielded::SerializedAction;

/// Builds the action from its transition
pub mod transformer;
mod v0;

pub use v0::*;

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Versioned `TokenMintToPool` action.
#[derive(Debug, Clone, From)]
pub enum TokenMintToPoolTransitionAction {
    /// Version 0
    V0(TokenMintToPoolTransitionActionV0),
}

impl TokenMintToPoolTransitionActionAccessorsV0 for TokenMintToPoolTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            Self::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            Self::V0(v0) => v0.base,
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            Self::V0(v0) => v0.amount(),
        }
    }
    fn public_note(&self) -> Option<&String> {
        match self {
            Self::V0(v0) => v0.public_note(),
        }
    }
    fn public_note_owned(self) -> Option<String> {
        match self {
            Self::V0(v0) => v0.public_note_owned(),
        }
    }

    fn actions(&self) -> &[SerializedAction] {
        match self {
            Self::V0(v0) => v0.actions(),
        }
    }

    fn anchor(&self) -> &[u8; 32] {
        match self {
            Self::V0(v0) => v0.anchor(),
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            Self::V0(v0) => v0.proof(),
        }
    }

    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            Self::V0(v0) => v0.binding_signature(),
        }
    }
}
