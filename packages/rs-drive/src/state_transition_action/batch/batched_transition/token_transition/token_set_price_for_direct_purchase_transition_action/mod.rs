use derive_more::From;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

/// transformer module for token issuance transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token issuance transition action
#[derive(Debug, Clone, From)]
pub enum TokenSetPriceForDirectPurchaseTransitionAction {
    /// v0
    V0(TokenSetPriceForDirectPurchaseTransitionActionV0),
}

impl TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0
    for TokenSetPriceForDirectPurchaseTransitionAction
{
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.base,
        }
    }

    fn price(&self) -> Option<&TokenPricingSchedule> {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.price.as_ref(),
        }
    }

    fn set_price(&mut self, price: Option<TokenPricingSchedule>) {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.price = price,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}
