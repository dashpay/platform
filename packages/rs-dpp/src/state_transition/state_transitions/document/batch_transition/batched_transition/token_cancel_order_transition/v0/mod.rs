pub mod v0_methods;

use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::tokens::token_order_side::TokenOrderSide;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The Identifier fields in [`TokenCancelOrderTransitionV0`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

/// Cancels the signer's own open order `order_id`: what it still holds in escrow goes back to
/// the signer and the order is deleted.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, DecodeUntrusted)]
// Auto-injects `json_safe_u64` on the u64 fields.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenCancelOrderTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// The order to cancel.
    pub order_id: Identifier,
    /// Whether the order is a listing or an offer.
    pub side: TokenOrderSide,
}

impl fmt::Display for TokenCancelOrderTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token CancelOrder, base: {}, order: {}, side: {}",
            self.base, self.order_id, self.side
        )
    }
}
