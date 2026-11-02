use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::shielded::SerializedAction;

/// transformer module
pub mod transformer;
mod v0;

pub use v0::*;

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Identity token balance -> token shielded pool. Carries the outputs-only Orchard bundle so state validation can verify the proof and lower it to note appends.
#[derive(Debug, Clone, From)]
pub enum TokenShieldTransitionAction {
    /// v0
    V0(TokenShieldTransitionActionV0),
}

impl TokenShieldTransitionActionAccessorsV0 for TokenShieldTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenShieldTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenShieldTransitionAction::V0(v0) => v0.base,
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            TokenShieldTransitionAction::V0(v0) => v0.amount(),
        }
    }

    fn actions(&self) -> &[SerializedAction] {
        match self {
            TokenShieldTransitionAction::V0(v0) => v0.actions(),
        }
    }

    fn anchor(&self) -> &[u8; 32] {
        match self {
            TokenShieldTransitionAction::V0(v0) => v0.anchor(),
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            TokenShieldTransitionAction::V0(v0) => v0.proof(),
        }
    }

    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            TokenShieldTransitionAction::V0(v0) => v0.binding_signature(),
        }
    }
}
