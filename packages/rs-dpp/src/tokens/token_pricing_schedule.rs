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
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(
        into = "TokenPricingScheduleRepr",
        from = "TokenPricingScheduleRepr"
    )
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

// Internal-`$type` serde shape. The tuple-variant outer enum can neither
// auto-derive internal tagging nor annotate its variant-internal u64s, so this
// struct-variant helper does both: `json_safe_u64` / `json_safe_u64_u64_map`
// keep the `Credits` and `TokenAmount` values JS-safe (string above
// `Number.MAX_SAFE_INTEGER`) in human-readable JSON, with no effect on `Value`
// or the bincode consensus path (which round-trips the outer enum directly).
#[cfg(feature = "serde-conversion")]
#[derive(Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "camelCase")]
enum TokenPricingScheduleRepr {
    SinglePrice {
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "crate::serialization::json_safe_u64")
        )]
        price: Credits,
    },
    SetPrices {
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "crate::serialization::json::safe_integer_map::json_safe_u64_u64_map")
        )]
        prices: BTreeMap<TokenAmount, Credits>,
    },
}

#[cfg(feature = "serde-conversion")]
impl From<TokenPricingSchedule> for TokenPricingScheduleRepr {
    fn from(schedule: TokenPricingSchedule) -> Self {
        match schedule {
            TokenPricingSchedule::SinglePrice(price) => Self::SinglePrice { price },
            TokenPricingSchedule::SetPrices(prices) => Self::SetPrices { prices },
        }
    }
}

#[cfg(feature = "serde-conversion")]
impl From<TokenPricingScheduleRepr> for TokenPricingSchedule {
    fn from(repr: TokenPricingScheduleRepr) -> Self {
        match repr {
            TokenPricingScheduleRepr::SinglePrice { price } => Self::SinglePrice(price),
            TokenPricingScheduleRepr::SetPrices { prices } => Self::SetPrices(prices),
        }
    }
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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::{platform_value, Value};
    use serde_json::json;

    // Internally `$type`-tagged: `SinglePrice(u64)` → `{"$type":"singlePrice",
    // "price": <n>}`, `SetPrices(BTreeMap<u64, u64>)` → `{"$type":"setPrices",
    // "prices": {<k>: <v>, ...}}`. `Credits`/`TokenAmount` u64s are JS-safe
    // (number below 2^53, string above); JSON forces map keys to strings while
    // platform_value preserves typed keys.

    #[test]
    fn json_round_trip_single_price() {
        use crate::serialization::JsonConvertible;
        let original = TokenPricingSchedule::SinglePrice(1234);
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({ "$type": "singlePrice", "price": 1234 }));
        let recovered = TokenPricingSchedule::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_single_price_above_max_safe_integer_is_string() {
        use crate::serialization::JsonConvertible;
        // Above Number.MAX_SAFE_INTEGER (2^53): json_safe_u64 must stringify the
        // Credits so JS consumers can't silently round it. (Raw u64 before the
        // Repr fix; string after.)
        let big: Credits = (1u64 << 53) + 1;
        let original = TokenPricingSchedule::SinglePrice(big);
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({ "$type": "singlePrice", "price": big.to_string() })
        );
        let recovered = TokenPricingSchedule::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_set_prices() {
        use crate::serialization::JsonConvertible;
        let mut prices = BTreeMap::new();
        prices.insert(5u64, 50u64);
        prices.insert(10u64, 80u64);
        let original = TokenPricingSchedule::SetPrices(prices);
        let json = original.to_json().expect("to_json");
        // JSON object keys must be strings — `serde_json` stringifies the
        // u64 amount keys.
        assert_eq!(
            json,
            json!({ "$type": "setPrices", "prices": { "5": 50, "10": 80 } })
        );
        let recovered = TokenPricingSchedule::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_single_price() {
        use crate::serialization::ValueConvertible;
        let original = TokenPricingSchedule::SinglePrice(1234);
        let value = original.to_object().expect("to_object");
        // `Credits` is `u64` → `Value::U64` (non-HR: json_safe_u64 stays typed).
        assert_eq!(
            value,
            platform_value!({ "$type": "singlePrice", "price": 1234u64 })
        );
        let recovered = TokenPricingSchedule::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_set_prices() {
        use crate::serialization::ValueConvertible;
        let mut prices = BTreeMap::new();
        prices.insert(5u64, 50u64);
        prices.insert(10u64, 80u64);
        let original = TokenPricingSchedule::SetPrices(prices);
        let value = original.to_object().expect("to_object");
        // platform_value preserves typed map keys: `BTreeMap<u64, u64>` →
        // map of `(Value::U64, Value::U64)` pairs. Serialized `$type` first.
        assert_eq!(
            value,
            Value::Map(vec![
                (
                    Value::Text("$type".to_string()),
                    Value::Text("setPrices".to_string()),
                ),
                (
                    Value::Text("prices".to_string()),
                    Value::Map(vec![
                        (Value::U64(5), Value::U64(50)),
                        (Value::U64(10), Value::U64(80)),
                    ]),
                ),
            ])
        );
        let recovered = TokenPricingSchedule::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
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
