use derive_more::From;
use dpp::identifier::Identifier;

/// transformer module for token freeze transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token freeze transition action
#[derive(Debug, Clone, From)]
pub enum TokenUnfreezeTransitionAction {
    /// v0
    V0(TokenUnfreezeTransitionActionV0),
}

impl TokenUnfreezeTransitionActionAccessorsV0 for TokenUnfreezeTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.base,
        }
    }

    fn frozen_identity_id(&self) -> Identifier {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.frozen_identity_id,
        }
    }

    fn set_frozen_identity_id(&mut self, id: Identifier) {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.frozen_identity_id = id,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}
