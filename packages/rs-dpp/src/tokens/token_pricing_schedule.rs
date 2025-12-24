use crate::balances::credits::TokenAmount;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use bincode_derive::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

/// Defines the pricing schedule for tokens in terms of credits.
///
/// A pricing schedule can either be a single, flat price applied to all
/// token amounts, or a tiered pricing model where specific amounts
/// correspond to specific credit values.
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
    /// A single flat price in credits for all token amounts.
    ///
    /// This variant is used when the pricing does not depend on
    /// the number of tokens being purchased or processed.
    SinglePrice(Credits),

    /// A tiered pricing model where specific token amounts map to credit prices.
    ///
    /// This allows for more complex pricing structures, such as
    /// volume discounts or progressive pricing. The map keys
    /// represent token amount thresholds, and the values are the
    /// corresponding credit prices.
    /// If the first token amount is greater than 1 this means that the user can only
    /// purchase that amount as a minimum at a time.
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
