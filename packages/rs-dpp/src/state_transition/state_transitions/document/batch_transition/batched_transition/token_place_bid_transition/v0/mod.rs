pub mod v0_methods;

use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
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
    /// The token's decimals the price is at. A offer executes only while they are the token's:
    /// the price is per whole token, so a change of the token's decimals between signing and
    /// execution would otherwise reprice it.
    pub decimals: u8,
    /// The smallest amount, in base units, a fill may take unless it takes everything still
    /// open; a fill may not leave less than this open either.
    pub min_fill: TokenAmount,
}

impl fmt::Display for TokenPlaceBidTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token PlaceBid, base: {}, amount: {}, price: {} at {} decimals, min fill: {}",
            self.base, self.amount, self.price, self.decimals, self.min_fill
        )
    }
}
