use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionTypeWithResolvedRecipient;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;

/// transformer module for token release transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token release transition action
#[derive(Debug, Clone, From)]
pub enum TokenReleaseTransitionAction {
    /// v0
    V0(TokenReleaseTransitionActionV0),
}

impl TokenReleaseTransitionActionAccessorsV0 for TokenReleaseTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenReleaseTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenReleaseTransitionAction::V0(v0) => v0.base,
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            TokenReleaseTransitionAction::V0(v0) => v0.amount,
        }
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenReleaseTransitionAction::V0(v0) => v0.amount = amount,
        }
    }

    fn recipient(&self) -> &TokenDistributionRecipient {
        match self {
            TokenReleaseTransitionAction::V0(v0) => &v0.recipient,
        }
    }

    fn recipient_owned(self) -> TokenDistributionRecipient {
        match self {
            TokenReleaseTransitionAction::V0(v0) => v0.recipient,
        }
    }

    fn set_recipient(&mut self, recipient: TokenDistributionRecipient) {
        match self {
            TokenReleaseTransitionAction::V0(v0) => v0.recipient = recipient,
        }
    }

    fn distribution_type_with_recipient(&self) -> &TokenDistributionTypeWithResolvedRecipient {
        match self {
            TokenReleaseTransitionAction::V0(v0) => &v0.distribution_type_with_recipient,
        }
    }

    fn set_distribution_type_with_recipient(&mut self, distribution_type_with_recipient: TokenDistributionTypeWithResolvedRecipient) {
        match self {
            TokenReleaseTransitionAction::V0(v0) => v0.distribution_type_with_recipient = distribution_type_with_recipient,
        }
    }
    
    fn public_note(&self) -> Option<&String> {
        match self {
            TokenReleaseTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenReleaseTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenReleaseTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}
