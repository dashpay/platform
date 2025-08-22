use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;

/// transformer module for token destroy_frozen_funds transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token destroy_frozen_funds transition action
#[derive(Debug, Clone, From)]
pub enum TokenDestroyFrozenFundsTransitionAction {
    /// v0
    V0(TokenDestroyFrozenFundsTransitionActionV0),
}

impl TokenDestroyFrozenFundsTransitionActionAccessorsV0
    for TokenDestroyFrozenFundsTransitionAction
{
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.base,
        }
    }

    fn frozen_identity_id(&self) -> Identifier {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.frozen_identity_id,
        }
    }

    fn set_frozen_identity_id(&mut self, id: Identifier) {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.frozen_identity_id = id,
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.amount,
        }
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.amount = amount,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}
