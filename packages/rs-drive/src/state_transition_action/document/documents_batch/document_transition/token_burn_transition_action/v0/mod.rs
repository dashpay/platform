mod transformer;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token burn transition action v0
#[derive(Debug, Clone)]
pub struct TokenBurnTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The amount of tokens to burn
    pub burn_amount: u64,
}

/// Accessors for `TokenBurnTransitionActionV0`
pub trait TokenBurnTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the amount of tokens to burn
    fn burn_amount(&self) -> u64;

    /// Sets the amount of tokens to burn
    fn set_burn_amount(&mut self, amount: u64);
}

impl TokenBurnTransitionActionAccessorsV0 for TokenBurnTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn burn_amount(&self) -> u64 {
        self.burn_amount
    }

    fn set_burn_amount(&mut self, amount: u64) {
        self.burn_amount = amount;
    }
}
