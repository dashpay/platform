use crate::balances::credits::TokenAmount;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "serde-conversion")]
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
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
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

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenPricingSchedule {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenPricingSchedule {}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_price_minimum_purchase_amount_and_price() {
        let schedule = TokenPricingSchedule::SinglePrice(500);
        let (amount, price) = schedule.minimum_purchase_amount_and_price();
        assert_eq!(amount, 1);
        assert_eq!(price, 500);
    }

    #[test]
    fn single_price_zero_credits() {
        let schedule = TokenPricingSchedule::SinglePrice(0);
        let (amount, price) = schedule.minimum_purchase_amount_and_price();
        assert_eq!(amount, 1);
        assert_eq!(price, 0);
    }

    #[test]
    fn set_prices_minimum_purchase_amount_and_price_single_entry() {
        let mut prices = BTreeMap::new();
        prices.insert(10u64, 100u64);
        let schedule = TokenPricingSchedule::SetPrices(prices);
        let (amount, price) = schedule.minimum_purchase_amount_and_price();
        assert_eq!(amount, 10);
        assert_eq!(price, 100);
    }

    #[test]
    fn set_prices_minimum_purchase_amount_and_price_multiple_entries() {
        let mut prices = BTreeMap::new();
        prices.insert(5u64, 50u64);
        prices.insert(10u64, 80u64);
        prices.insert(100u64, 500u64);
        let schedule = TokenPricingSchedule::SetPrices(prices);
        // BTreeMap orders by key, so the first entry is the minimum amount
        let (amount, price) = schedule.minimum_purchase_amount_and_price();
        assert_eq!(amount, 5);
        assert_eq!(price, 50);
    }

    #[test]
    fn set_prices_empty_map_returns_default() {
        let prices = BTreeMap::new();
        let schedule = TokenPricingSchedule::SetPrices(prices);
        let (amount, price) = schedule.minimum_purchase_amount_and_price();
        // unwrap_or_default returns (0, 0) for empty map
        assert_eq!(amount, 0);
        assert_eq!(price, 0);
    }

    #[test]
    fn display_single_price() {
        let schedule = TokenPricingSchedule::SinglePrice(1234);
        assert_eq!(format!("{}", schedule), "SinglePrice: 1234");
    }

    #[test]
    fn display_set_prices_empty() {
        let schedule = TokenPricingSchedule::SetPrices(BTreeMap::new());
        assert_eq!(format!("{}", schedule), "SetPrices: []");
    }

    #[test]
    fn display_set_prices_single_entry() {
        let mut prices = BTreeMap::new();
        prices.insert(10u64, 100u64);
        let schedule = TokenPricingSchedule::SetPrices(prices);
        assert_eq!(format!("{}", schedule), "SetPrices: [10 => 100]");
    }

    #[test]
    fn display_set_prices_multiple_entries() {
        let mut prices = BTreeMap::new();
        prices.insert(5u64, 50u64);
        prices.insert(10u64, 80u64);
        let schedule = TokenPricingSchedule::SetPrices(prices);
        // BTreeMap iterates in sorted key order
        assert_eq!(format!("{}", schedule), "SetPrices: [5 => 50, 10 => 80]");
    }
}
