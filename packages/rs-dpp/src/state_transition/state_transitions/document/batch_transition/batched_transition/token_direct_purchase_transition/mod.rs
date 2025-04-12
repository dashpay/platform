pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenDirectPurchaseTransitionV0;

/// Represents a versioned transition for direct token purchases.
///
/// This enum allows for forward-compatible support of different versions
/// of the `TokenDirectPurchaseTransition` structure. Each variant corresponds
/// to a specific version of the transition logic and structure.
///
/// This transition type is used when a user intends to directly purchase tokens
/// by specifying the desired amount and the maximum total price they are willing to pay.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenDirectPurchaseTransition {
    /// Version 0 of the token direct purchase transition.
    ///
    /// This version includes the base document transition, the number of tokens
    /// to purchase, and the maximum total price the user agrees to pay.
    /// If the price in the contract is lower than the agreed price, the lower
    /// price is used.
    #[display("V0({})", "_0")]
    V0(TokenDirectPurchaseTransitionV0),
}

impl Default for TokenDirectPurchaseTransition {
    fn default() -> Self {
        TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0::default())
        // since only v0
    }
}
