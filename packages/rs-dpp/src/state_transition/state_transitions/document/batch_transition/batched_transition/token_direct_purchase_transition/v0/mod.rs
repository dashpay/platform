pub mod v0_methods;

use crate::balances::credits::{Credits, TokenAmount};
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, Encode};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The Identifier fields in [`TokenDirectPurchaseTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct TokenDirectPurchaseTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// How many tokens should we buy.
    pub token_count: TokenAmount,
    /// Price that the user is willing to pay for all the tokens.
    /// The user will pay up to this amount.
    /// If the actual cost of the token per the contract is less than the agreed price that the user is willing to pay
    /// Then we take the actual cost per the contract.
    pub total_agreed_price: Credits,
}

impl fmt::Display for TokenDirectPurchaseTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token DirectPurchase, base: {}, token count: {}, price {}",
            self.base, self.token_count, self.total_agreed_price
        )
    }
}
