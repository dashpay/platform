use derive_more::From;
use dpp::shielded::SerializedAction;

/// transformer module
pub mod transformer;
mod v0;

pub use v0::*;

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// A transfer inside the token's shielded pool. Carries the Orchard spend bundle (value balance zero) so state validation can verify the proof and lower it to nullifier inserts and note appends.
#[derive(Debug, Clone, From)]
pub enum TokenShieldedTransferTransitionAction {
    /// v0
    V0(TokenShieldedTransferTransitionActionV0),
}

impl TokenShieldedTransferTransitionActionAccessorsV0 for TokenShieldedTransferTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenShieldedTransferTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenShieldedTransferTransitionAction::V0(v0) => v0.base,
        }
    }

    fn actions(&self) -> &[SerializedAction] {
        match self {
            TokenShieldedTransferTransitionAction::V0(v0) => v0.actions(),
        }
    }

    fn anchor(&self) -> &[u8; 32] {
        match self {
            TokenShieldedTransferTransitionAction::V0(v0) => v0.anchor(),
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            TokenShieldedTransferTransitionAction::V0(v0) => v0.proof(),
        }
    }

    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            TokenShieldedTransferTransitionAction::V0(v0) => v0.binding_signature(),
        }
    }
}
