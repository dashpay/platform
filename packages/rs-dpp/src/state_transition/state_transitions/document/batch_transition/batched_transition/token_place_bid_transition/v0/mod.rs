pub mod v0_methods;

use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::prelude::TimestampMillis;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, DecodeUntrusted, Encode};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The Identifier fields in [`TokenPlaceBidTransitionV0`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

/// Offers to buy `amount` of a tradeable token at `price` credits per whole token. What the whole
/// amount costs at that price moves from the buyer's balance into escrow, where it stays until
/// the offer is filled or cancelled.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, DecodeUntrusted)]
// Auto-injects `json_safe_u64` on the u64 fields.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenPlaceBidTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// The token amount wanted, in base units.
    pub amount: TokenAmount,
    /// The credits offered per whole token (10 to the power of the token's decimals base units).
    pub price: Credits,
    /// The smallest amount, in base units, a fill may take unless it takes everything still
    /// open; a fill may not leave less than this open either.
    pub min_fill: TokenAmount,
    /// The block time from which the offer can no longer be filled. It can still be cancelled.
    pub expires_at: Option<TimestampMillis>,
}

impl fmt::Display for TokenPlaceBidTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token PlaceBid, base: {}, amount: {}, price: {}, min fill: {}, expires at: {:?}",
            self.base, self.amount, self.price, self.min_fill, self.expires_at
        )
    }
}
