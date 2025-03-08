use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionInfo;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;

/// transformer module for token release transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token release transition action
#[derive(Debug, Clone, From)]
pub enum TokenClaimTransitionAction {
    /// v0
    V0(TokenClaimTransitionActionV0),
}

impl TokenClaimTransitionActionAccessorsV0 for TokenClaimTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenClaimTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenClaimTransitionAction::V0(v0) => v0.base,
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            TokenClaimTransitionAction::V0(v0) => v0.amount,
        }
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenClaimTransitionAction::V0(v0) => v0.amount = amount,
        }
    }

    fn recipient(&self) -> TokenDistributionRecipient {
        match self {
            TokenClaimTransitionAction::V0(v0) => v0.recipient(),
        }
    }

    fn distribution_info(&self) -> &TokenDistributionInfo {
        match self {
            TokenClaimTransitionAction::V0(v0) => &v0.distribution_info,
        }
    }

    fn set_distribution_info(&mut self, distribution_info: TokenDistributionInfo) {
        match self {
            TokenClaimTransitionAction::V0(v0) => v0.distribution_info = distribution_info,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenClaimTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenClaimTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenClaimTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}
