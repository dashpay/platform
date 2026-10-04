use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use dpp::shielded::SerializedAction;

/// transformer module
pub mod transformer;
mod v0;

pub use v0::*;

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token shielded pool -> identity token balance. Carries the Orchard spend bundle so state validation can verify the proof and lower it to nullifier inserts, change-note appends and the recipient credit.
#[derive(Debug, Clone, From)]
pub enum TokenUnshieldTransitionAction {
    /// v0
    V0(TokenUnshieldTransitionActionV0),
}

impl TokenUnshieldTransitionActionAccessorsV0 for TokenUnshieldTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenUnshieldTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenUnshieldTransitionAction::V0(v0) => v0.base,
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            TokenUnshieldTransitionAction::V0(v0) => v0.amount(),
        }
    }

    fn recipient_id(&self) -> Identifier {
        match self {
            TokenUnshieldTransitionAction::V0(v0) => v0.recipient_id(),
        }
    }

    fn actions(&self) -> &[SerializedAction] {
        match self {
            TokenUnshieldTransitionAction::V0(v0) => v0.actions(),
        }
    }

    fn anchor(&self) -> &[u8; 32] {
        match self {
            TokenUnshieldTransitionAction::V0(v0) => v0.anchor(),
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            TokenUnshieldTransitionAction::V0(v0) => v0.proof(),
        }
    }

    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            TokenUnshieldTransitionAction::V0(v0) => v0.binding_signature(),
        }
    }
}
