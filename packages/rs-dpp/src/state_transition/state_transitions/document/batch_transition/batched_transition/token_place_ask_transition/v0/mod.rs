pub mod v0_methods;

use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::prelude::TimestampMillis;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, DecodeUntrusted, Encode};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The Identifier fields in [`TokenPlaceAskTransitionV0`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

/// Lists `amount` of a tradeable token for sale at `price` credits per whole token. The amount
/// moves from the seller's balance into escrow, where it stays until the listing is filled or
/// cancelled.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, DecodeUntrusted)]
// Auto-injects `json_safe_u64` on the u64 fields.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenPlaceAskTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// The token amount listed, in base units.
    pub amount: TokenAmount,
    /// The credits asked per whole token (10 to the power of the token's decimals base units).
    pub price: Credits,
    /// The smallest amount, in base units, a fill may take unless it takes everything still
    /// open; a fill may not leave less than this open either. Equal to `amount` for a listing
    /// that is sold all at once or not at all.
    pub min_fill: TokenAmount,
    /// The block time from which the listing can no longer be filled. It can still be
    /// cancelled.
    pub expires_at: Option<TimestampMillis>,
}

impl fmt::Display for TokenPlaceAskTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token PlaceAsk, base: {}, amount: {}, price: {}, min fill: {}, expires at: {:?}",
            self.base, self.amount, self.price, self.min_fill, self.expires_at
        )
    }
}
