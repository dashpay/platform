use crate::balances::credits::TokenAmount;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use bincode_derive::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    PlatformSerialize,
    PlatformDeserialize,
)]
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
            TokenPricingSchedule::SetPrices(prices) => prices
                .first_key_value()
                .map(|(amount, cost)| (*amount, *cost))
                .unwrap_or_default(),
        }
    }
}

impl Display for TokenPricingSchedule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TokenPricingSchedule::SinglePrice(credits) => {
                write!(f, "SinglePrice: {}", credits)
            }
            TokenPricingSchedule::SetPrices(prices) => {
                write!(f, "SetPrices: [")?;
                for (i, (amount, credits)) in prices.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} => {}", amount, credits)?;
                }
                write!(f, "]")
            }
        }
    }
}
