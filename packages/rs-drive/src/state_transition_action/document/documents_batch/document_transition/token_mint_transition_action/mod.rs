use derive_more::From;
use dpp::identifier::Identifier;

/// transformer module for token issuance transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token issuance transition action
#[derive(Debug, Clone, From)]
pub enum TokenMintTransitionAction {
    /// v0
    V0(TokenIssuanceTransitionActionV0),
}

impl TokenMintTransitionActionAccessorsV0 for TokenMintTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenMintTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.base,
        }
    }

    fn issuance_amount(&self) -> u64 {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.issuance_amount,
        }
    }

    fn set_issuance_amount(&mut self, amount: u64) {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.issuance_amount = amount,
        }
    }

    fn identity_balance_holder_id(&self) -> Identifier {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.identity_balance_holder_id,
        }
    }

    fn set_identity_balance_holder_id(&mut self, id: Identifier) {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.identity_balance_holder_id = id,
        }
    }
}
