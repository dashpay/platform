mod transformer;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessors};

/// Token issuance transition action v0
#[derive(Debug, Clone)]
pub struct TokenIssuanceTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The amount of tokens to create
    pub issuance_amount: u64,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenIssuanceTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the amount of tokens to issuance
    fn issuance_amount(&self) -> u64;

    /// Sets the amount of tokens to issuance
    fn set_issuance_amount(&mut self, amount: u64);
}

impl TokenIssuanceTransitionActionAccessorsV0 for TokenIssuanceTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn issuance_amount(&self) -> u64 {
        self.issuance_amount
    }

    fn set_issuance_amount(&mut self, amount: u64) {
        self.issuance_amount = amount;
    }
}
