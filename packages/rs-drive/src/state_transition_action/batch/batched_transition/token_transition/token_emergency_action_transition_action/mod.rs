use derive_more::From;
use dpp::tokens::emergency_action::TokenEmergencyAction;

/// transformer module for token emergency_action transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token emergency_action transition action
#[derive(Debug, Clone, From)]
pub enum TokenEmergencyActionTransitionAction {
    /// v0
    V0(TokenEmergencyActionTransitionActionV0),
}

impl TokenEmergencyActionTransitionActionAccessorsV0 for TokenEmergencyActionTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.base,
        }
    }

    fn emergency_action(&self) -> TokenEmergencyAction {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.emergency_action(),
        }
    }

    fn set_emergency_action(&mut self, emergency_action: TokenEmergencyAction) {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => {
                v0.set_emergency_action(emergency_action)
            }
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}
