pub mod v0_methods;

use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The Identifier fields in [`TokenFillBidTransitionV0`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

/// Sells into the open offer `order_id`: everything it still wants, up to `max_amount`, and at
/// least `min_amount`. The seller's tokens move to the buyer, and the seller is paid the offer's
/// price for what the fill takes out of its escrow, refused when that is below
/// `min_total_received`.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, DecodeUntrusted)]
// Auto-injects `json_safe_u64` on the u64 fields.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenFillBidTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// The offer to sell into.
    pub order_id: Identifier,
    /// The least amount, in base units, the seller accepts to sell.
    pub min_amount: TokenAmount,
    /// The most amount, in base units, the seller sells.
    pub max_amount: TokenAmount,
    /// The fewest credits the seller accepts for the fill.
    pub min_total_received: Credits,
}

impl fmt::Display for TokenFillBidTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token FillBid, base: {}, order: {}, min amount: {}, max amount: {}, min total received: {}",
            self.base, self.order_id, self.min_amount, self.max_amount, self.min_total_received
        )
    }
}
