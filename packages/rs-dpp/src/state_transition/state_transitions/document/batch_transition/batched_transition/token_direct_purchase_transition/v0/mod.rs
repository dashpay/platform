pub mod v0_methods;

use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, Encode};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::balances::credits::TokenAmount;
use crate::fee::Credits;

/// The Identifier fields in [`TokenDirectPurchaseTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenDirectPurchaseTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// How many tokens should we buy.
    pub token_count: TokenAmount,
    /// Agreed price per token
    /// The user will pay this amount times the token count
    pub agreed_price_per_token: Credits,
}

impl fmt::Display for TokenDirectPurchaseTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token DirectPurchase, base: {}, token count: {}, price per token {}",
            self.base,
            self.token_count,
            self.agreed_price_per_token
        )
    }
}
