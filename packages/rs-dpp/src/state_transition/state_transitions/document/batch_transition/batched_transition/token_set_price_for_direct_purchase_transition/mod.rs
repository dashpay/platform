pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenSetPriceForDirectPurchaseTransitionV0;

/// Represents a versioned transition for setting or updating the price of a token
/// available for direct purchase.
///
/// This transition allows a token owner or controlling group to define or remove a pricing
/// schedule for direct purchases. Setting the price to `None` disables further purchases
/// of the token.
///
/// This transition type supports **group actions**, meaning it can require **multi-signature
/// (multisig) authorization**. In such cases, multiple identities must agree and sign
/// the transition for it to be considered valid and executable.
///
/// Versioning enables forward compatibility by allowing future enhancements or changes
/// without breaking existing clients.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenSetPriceForDirectPurchaseTransition {
    /// Version 0 of the token set price for direct purchase transition.
    ///
    /// This version includes:
    /// - A base document transition.
    /// - An optional pricing schedule: `Some(...)` to set the token's price, or `None` to make it non-purchasable.
    /// - An optional public note.
    ///
    /// Group actions with multisig are supported in this version,
    /// enabling shared control over token pricing among multiple authorized identities.
    #[display("V0({})", "_0")]
    V0(TokenSetPriceForDirectPurchaseTransitionV0),
}

impl Default for TokenSetPriceForDirectPurchaseTransition {
    fn default() -> Self {
        TokenSetPriceForDirectPurchaseTransition::V0(
            TokenSetPriceForDirectPurchaseTransitionV0::default(),
        ) // since only v0
    }
}
