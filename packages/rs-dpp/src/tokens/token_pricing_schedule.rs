use std::collections::BTreeMap;
use bincode_derive::{Decode, Encode};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use crate::balances::credits::TokenAmount;
use crate::fee::Credits;

#[derive(Debug, Clone, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, Display, PlatformSerialize, PlatformDeserialize)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenPricingSchedule {
    SinglePrice(Credits),
    SetPrices(BTreeMap<TokenAmount, Credits>),
}

impl TokenPricingSchedule {
    pub fn minimum_purchase_amount_and_price(&self) -> (TokenAmount, Credits) {
        match self {
            TokenPricingSchedule::SinglePrice(price) => (1, *price),
            TokenPricingSchedule::SetPrices(prices) => prices.first_key_value().map(|(amount, cost)| (*amount, *cost)).unwrap_or_default()
        }
    }
}
