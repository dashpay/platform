pub mod v0_methods;

/// The Identifier fields in [`TokenSetPriceForDirectPurchaseTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
use bincode::{Decode, Encode};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenSetPriceForDirectPurchaseTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "issuedToIdentityId")
    )]
    /// What should be the price for a single token
    /// Setting this to None makes it no longer purchasable
    pub price: Option<TokenPricingSchedule>,
    /// The public note
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "publicNote")
    )]
    pub public_note: Option<String>,
}

impl fmt::Display for TokenSetPriceForDirectPurchaseTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let price_str = match &self.price {
            Some(p) => p.to_string(),
            None => "None".to_string(),
        };

        let note_str = match &self.public_note {
            Some(note) => note,
            None => "None",
        };

        write!(
            f,
            "Token Set Price for Direct Purchase, base: {}, price: {}, public note: {}",
            self.base, price_str, note_str
        )
    }
}
