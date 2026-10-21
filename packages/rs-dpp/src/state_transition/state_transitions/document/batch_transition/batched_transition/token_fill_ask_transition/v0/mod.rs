pub mod v0_methods;

use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The Identifier fields in [`TokenFillAskTransitionV0`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

/// Buys from the open listing `order_id`: everything still open, up to `max_amount`, and at
/// least `min_amount`. The buyer pays the listing's price for what the fill takes, refused when
/// that is above `max_total_price`; the tokens move from escrow to the buyer.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, DecodeUntrusted)]
// Auto-injects `json_safe_u64` on the u64 fields.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenFillAskTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// The listing to buy from.
    pub order_id: Identifier,
    /// The least amount, in base units, the buyer accepts; less still open refuses the fill.
    pub min_amount: TokenAmount,
    /// The most amount, in base units, the buyer takes.
    pub max_amount: TokenAmount,
    /// The most credits the buyer pays for the fill.
    pub max_total_price: Credits,
}

impl fmt::Display for TokenFillAskTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token FillAsk, base: {}, order: {}, min amount: {}, max amount: {}, max total price: {}",
            self.base, self.order_id, self.min_amount, self.max_amount, self.max_total_price
        )
    }
}
