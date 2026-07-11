use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use bincode::{BorrowDecode, Decode, Encode};
use std::collections::BTreeMap;

impl Encode for DistributionFunction {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        match self {
            DistributionFunction::FixedAmount { amount: n } => {
                0u8.encode(encoder)?;
                n.encode(encoder)?;
            }
            DistributionFunction::Random { min, max } => {
                1u8.encode(encoder)?;
                min.encode(encoder)?;
                max.encode(encoder)?;
            }
            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                start_decreasing_offset: s,
                max_interval_count,
                distribution_start_amount: n,
                trailing_distribution_interval_amount,
                min_value,
            } => {
                2u8.encode(encoder)?;
                step_count.encode(encoder)?;
                decrease_per_interval_numerator.encode(encoder)?;
                decrease_per_interval_denominator.encode(encoder)?;
                s.encode(encoder)?;
                max_interval_count.encode(encoder)?;
                n.encode(encoder)?;
                trailing_distribution_interval_amount.encode(encoder)?;
                min_value.encode(encoder)?;
            }
            DistributionFunction::Stepwise(steps) => {
                3u8.encode(encoder)?;
                steps.encode(encoder)?;
            }
            DistributionFunction::Linear {
                a,
                d,
                start_step: s,
                starting_amount: b,
                min_value,
                max_value,
            } => {
                4u8.encode(encoder)?;
                a.encode(encoder)?;
                d.encode(encoder)?;
                s.encode(encoder)?;
                b.encode(encoder)?;
                min_value.encode(encoder)?;
                max_value.encode(encoder)?;
            }
            DistributionFunction::Polynomial {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            } => {
                5u8.encode(encoder)?;
                a.encode(encoder)?;
                d.encode(encoder)?;
                m.encode(encoder)?;
                n.encode(encoder)?;
                o.encode(encoder)?;
                start_moment.encode(encoder)?;
                b.encode(encoder)?;
                min_value.encode(encoder)?;
                max_value.encode(encoder)?;
            }
            DistributionFunction::Exponential {
                a,
                d,
                m,
                n,
                o,
                start_moment: s,
                b,
                min_value,
                max_value,
            } => {
                6u8.encode(encoder)?;
                a.encode(encoder)?;
                d.encode(encoder)?;
                m.encode(encoder)?;
                n.encode(encoder)?;
                o.encode(encoder)?;
                s.encode(encoder)?;
                b.encode(encoder)?;
                min_value.encode(encoder)?;
                max_value.encode(encoder)?;
            }
            DistributionFunction::Logarithmic {
                a,
                d,
                m,
                n,
                o,
                start_moment: s,
                b,
                min_value,
                max_value,
            } => {
                7u8.encode(encoder)?;
                a.encode(encoder)?;
                d.encode(encoder)?;
                m.encode(encoder)?;
                n.encode(encoder)?;
                o.encode(encoder)?;
                s.encode(encoder)?;
                b.encode(encoder)?;
                min_value.encode(encoder)?;
                max_value.encode(encoder)?;
            }
            DistributionFunction::InvertedLogarithmic {
                a,
                d,
                m,
                n,
                o,
                start_moment: s,
                b,
                min_value,
                max_value,
            } => {
                8u8.encode(encoder)?;
                a.encode(encoder)?;
                d.encode(encoder)?;
                m.encode(encoder)?;
                n.encode(encoder)?;
                o.encode(encoder)?;
                s.encode(encoder)?;
                b.encode(encoder)?;
                min_value.encode(encoder)?;
                max_value.encode(encoder)?;
            }
        }
        Ok(())
    }
}

impl<C> Decode<C> for DistributionFunction {
    fn decode<D: bincode::de::Decoder<Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let variant = u8::decode(decoder)?;
        match variant {
            0 => {
                let n = TokenAmount::decode(decoder)?;
                Ok(Self::FixedAmount { amount: n })
            }
            1 => {
                let min = TokenAmount::decode(decoder)?;
                let max = TokenAmount::decode(decoder)?;
                Ok(Self::Random { min, max })
            }
            2 => {
                let step_count = u32::decode(decoder)?;
                let decrease_per_interval_numerator = u16::decode(decoder)?;
                let decrease_per_interval_denominator = u16::decode(decoder)?;
                let s = Option::<u64>::decode(decoder)?;
                let max_interval_count = Option::<u16>::decode(decoder)?;
                let n = TokenAmount::decode(decoder)?;
                let trailing_distribution_interval_amount = TokenAmount::decode(decoder)?;
                let min_value = Option::<u64>::decode(decoder)?;
                Ok(Self::StepDecreasingAmount {
                    start_decreasing_offset: s,
                    decrease_per_interval_numerator,
                    decrease_per_interval_denominator,
                    step_count,
                    distribution_start_amount: n,
                    max_interval_count,
                    min_value,
                    trailing_distribution_interval_amount,
                })
            }
            3 => {
                let steps = BTreeMap::<u64, TokenAmount>::decode(decoder)?;
                Ok(Self::Stepwise(steps))
            }
            4 => {
                let a = i64::decode(decoder)?;
                let d = u64::decode(decoder)?;
                let s = Option::<u64>::decode(decoder)?;
                let b = TokenAmount::decode(decoder)?;
                let min_value = Option::<u64>::decode(decoder)?;
                let max_value = Option::<u64>::decode(decoder)?;
                Ok(Self::Linear {
                    a,
                    d,
                    start_step: s,
                    starting_amount: b,
                    min_value,
                    max_value,
                })
            }
            5 => {
                let a = i64::decode(decoder)?;
                let d = u64::decode(decoder)?;
                let m = i64::decode(decoder)?;
                let n = u64::decode(decoder)?;
                let o = i64::decode(decoder)?;
                let s = Option::<u64>::decode(decoder)?;
                let b = TokenAmount::decode(decoder)?;
                let min_value = Option::<u64>::decode(decoder)?;
                let max_value = Option::<u64>::decode(decoder)?;
                Ok(Self::Polynomial {
                    a,
                    d,
                    m,
                    n,
                    o,
                    start_moment: s,
                    b,
                    min_value,
                    max_value,
                })
            }
            6 => {
                let a = u64::decode(decoder)?;
                let d = u64::decode(decoder)?;
                let m = i64::decode(decoder)?;
                let n = u64::decode(decoder)?;
                let o = i64::decode(decoder)?;
                let start_moment = Option::<u64>::decode(decoder)?;
                let b = TokenAmount::decode(decoder)?;
                let min_value = Option::<u64>::decode(decoder)?;
                let max_value = Option::<u64>::decode(decoder)?;
                Ok(Self::Exponential {
                    a,
                    d,
                    m,
                    n,
                    o,
                    start_moment,
                    b,
                    min_value,
                    max_value,
                })
            }
            7 => {
                let a = i64::decode(decoder)?;
                let d = u64::decode(decoder)?;
                let m = u64::decode(decoder)?;
                let n = u64::decode(decoder)?;
                let o = i64::decode(decoder)?;
                let s = Option::<u64>::decode(decoder)?;
                let b = TokenAmount::decode(decoder)?;
                let min_value = Option::<u64>::decode(decoder)?;
                let max_value = Option::<u64>::decode(decoder)?;
                Ok(Self::Logarithmic {
                    a,
                    d,
                    m,
                    n,
                    o,
                    start_moment: s,
                    b,
                    min_value,
                    max_value,
                })
            }
            8 => {
                let a = i64::decode(decoder)?;
                let d = u64::decode(decoder)?;
                let m = u64::decode(decoder)?;
                let n = u64::decode(decoder)?;
                let o = i64::decode(decoder)?;
                let s = Option::<u64>::decode(decoder)?;
                let b = TokenAmount::decode(decoder)?;
                let min_value = Option::<u64>::decode(decoder)?;
                let max_value = Option::<u64>::decode(decoder)?;
                Ok(Self::InvertedLogarithmic {
                    a,
                    d,
                    m,
                    n,
                    o,
                    start_moment: s,
                    b,
                    min_value,
                    max_value,
                })
            }
            _ => Err(bincode::error::DecodeError::OtherString(
                "Invalid variant".into(),
            )),
        }
    }
}

impl<'de, C> BorrowDecode<'de, C> for DistributionFunction {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let variant = u8::borrow_decode(decoder)?;
        match variant {
            0 => {
                let n = TokenAmount::borrow_decode(decoder)?;
                Ok(Self::FixedAmount { amount: n })
            }
            1 => {
                let min = TokenAmount::borrow_decode(decoder)?;
                let max = TokenAmount::borrow_decode(decoder)?;
                Ok(Self::Random { min, max })
            }
            2 => {
                let step_count = u32::borrow_decode(decoder)?;
                let decrease_per_interval_numerator = u16::borrow_decode(decoder)?;
                let decrease_per_interval_denominator = u16::borrow_decode(decoder)?;
                let s = Option::<u64>::borrow_decode(decoder)?;
                let max_interval_count = Option::<u16>::borrow_decode(decoder)?;
                let n = TokenAmount::borrow_decode(decoder)?;
                let trailing_distribution_interval_amount = TokenAmount::borrow_decode(decoder)?;
                let min_value = Option::<u64>::borrow_decode(decoder)?;
                Ok(Self::StepDecreasingAmount {
                    step_count,
                    decrease_per_interval_numerator,
                    decrease_per_interval_denominator,
                    start_decreasing_offset: s,
                    max_interval_count,
                    distribution_start_amount: n,
                    trailing_distribution_interval_amount,
                    min_value,
                })
            }
            3 => {
                let steps = BTreeMap::<u64, TokenAmount>::borrow_decode(decoder)?;
                Ok(Self::Stepwise(steps))
            }
            4 => {
                let a = i64::borrow_decode(decoder)?;
                let d = u64::borrow_decode(decoder)?;
                let s = Option::<u64>::borrow_decode(decoder)?;
                let b = TokenAmount::borrow_decode(decoder)?;
                let min_value = Option::<u64>::borrow_decode(decoder)?;
                let max_value = Option::<u64>::borrow_decode(decoder)?;
                Ok(Self::Linear {
                    a,
                    d,
                    start_step: s,
                    starting_amount: b,
                    min_value,
                    max_value,
                })
            }
            5 => {
                let a = i64::borrow_decode(decoder)?;
                let d = u64::borrow_decode(decoder)?;
                let m = i64::borrow_decode(decoder)?;
                let n = u64::borrow_decode(decoder)?;
                let o = i64::borrow_decode(decoder)?;
                let s = Option::<u64>::borrow_decode(decoder)?;
                let b = TokenAmount::borrow_decode(decoder)?;
                let min_value = Option::<u64>::borrow_decode(decoder)?;
                let max_value = Option::<u64>::borrow_decode(decoder)?;
                Ok(Self::Polynomial {
                    a,
                    d,
                    m,
                    n,
                    o,
                    start_moment: s,
                    b,
                    min_value,
                    max_value,
                })
            }
            6 => {
                let a = u64::borrow_decode(decoder)?;
                let d = u64::borrow_decode(decoder)?;
                let m = i64::borrow_decode(decoder)?;
                let n = u64::borrow_decode(decoder)?;
                let o = i64::borrow_decode(decoder)?;
                let start_moment = Option::<u64>::borrow_decode(decoder)?;
                let b = TokenAmount::borrow_decode(decoder)?;
                let min_value = Option::<u64>::borrow_decode(decoder)?;
                let max_value = Option::<u64>::borrow_decode(decoder)?;
                Ok(Self::Exponential {
                    a,
                    d,
                    m,
                    n,
                    o,
                    start_moment,
                    b,
                    min_value,
                    max_value,
                })
            }
            7 => {
                let a = i64::borrow_decode(decoder)?;
                let d = u64::borrow_decode(decoder)?;
                let m = u64::borrow_decode(decoder)?;
                let n = u64::borrow_decode(decoder)?;
                let o = i64::borrow_decode(decoder)?;
                let s = Option::<u64>::borrow_decode(decoder)?;
                let b = TokenAmount::borrow_decode(decoder)?;
                let min_value = Option::<u64>::borrow_decode(decoder)?;
                let max_value = Option::<u64>::borrow_decode(decoder)?;
                Ok(Self::Logarithmic {
                    a,
                    d,
                    m,
                    n,
                    o,
                    start_moment: s,
                    b,
                    min_value,
                    max_value,
                })
            }
            8 => {
                let a = i64::borrow_decode(decoder)?;
                let d = u64::borrow_decode(decoder)?;
                let m = u64::borrow_decode(decoder)?;
                let n = u64::borrow_decode(decoder)?;
                let o = i64::borrow_decode(decoder)?;
                let s = Option::<u64>::borrow_decode(decoder)?;
                let b = TokenAmount::borrow_decode(decoder)?;
                let min_value = Option::<u64>::borrow_decode(decoder)?;
                let max_value = Option::<u64>::borrow_decode(decoder)?;
                Ok(Self::InvertedLogarithmic {
                    a,
                    d,
                    m,
                    n,
                    o,
                    start_moment: s,
                    b,
                    min_value,
                    max_value,
                })
            }
            _ => Err(bincode::error::DecodeError::OtherString(
                "Invalid variant".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: bincode::config::Configuration = bincode::config::standard();

    /// Helper: encode then decode a DistributionFunction and assert round-trip equality.
    fn round_trip(original: &DistributionFunction) -> DistributionFunction {
        let bytes = bincode::encode_to_vec(original, CONFIG).expect("encode failed");
        let (decoded, _): (DistributionFunction, _) =
            bincode::decode_from_slice(&bytes, CONFIG).expect("decode failed");
        decoded
    }

    /// Helper: encode then borrow-decode a DistributionFunction and assert round-trip equality.
    fn round_trip_borrow(original: &DistributionFunction) -> DistributionFunction {
        let bytes = bincode::encode_to_vec(original, CONFIG).expect("encode failed");
        let (decoded, _): (DistributionFunction, _) =
            bincode::borrow_decode_from_slice(&bytes, CONFIG).expect("borrow_decode failed");
        decoded
    }

    // -----------------------------------------------------------------------
    // Round-trip tests for each variant
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_fixed_amount() {
        let original = DistributionFunction::FixedAmount { amount: 42 };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_random() {
        let original = DistributionFunction::Random { min: 10, max: 100 };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_step_decreasing_amount() {
        let original = DistributionFunction::StepDecreasingAmount {
            step_count: 210_000,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 2,
            start_decreasing_offset: Some(100),
            max_interval_count: Some(64),
            distribution_start_amount: 5000,
            trailing_distribution_interval_amount: 1,
            min_value: Some(10),
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_step_decreasing_amount_none_options() {
        let original = DistributionFunction::StepDecreasingAmount {
            step_count: 1000,
            decrease_per_interval_numerator: 7,
            decrease_per_interval_denominator: 100,
            start_decreasing_offset: None,
            max_interval_count: None,
            distribution_start_amount: 999,
            trailing_distribution_interval_amount: 0,
            min_value: None,
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_stepwise() {
        let mut steps = BTreeMap::new();
        steps.insert(0, 100);
        steps.insert(10, 50);
        steps.insert(20, 25);
        let original = DistributionFunction::Stepwise(steps);
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_stepwise_empty() {
        let original = DistributionFunction::Stepwise(BTreeMap::new());
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_linear() {
        let original = DistributionFunction::Linear {
            a: -5,
            d: 100,
            start_step: Some(10),
            starting_amount: 1000,
            min_value: Some(50),
            max_value: Some(2000),
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_linear_none_options() {
        let original = DistributionFunction::Linear {
            a: 3,
            d: 1,
            start_step: None,
            starting_amount: 500,
            min_value: None,
            max_value: None,
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_polynomial() {
        let original = DistributionFunction::Polynomial {
            a: -3,
            d: 10,
            m: 2,
            n: 1,
            o: -1,
            start_moment: Some(5),
            b: 100,
            min_value: Some(0),
            max_value: Some(10000),
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_polynomial_none_options() {
        let original = DistributionFunction::Polynomial {
            a: 1,
            d: 1,
            m: -2,
            n: 3,
            o: 0,
            start_moment: None,
            b: 50,
            min_value: None,
            max_value: None,
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_exponential() {
        let original = DistributionFunction::Exponential {
            a: 100,
            d: 20,
            m: -3,
            n: 100,
            o: 5,
            start_moment: Some(10),
            b: 10,
            min_value: Some(1),
            max_value: Some(500),
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_exponential_none_options() {
        let original = DistributionFunction::Exponential {
            a: 50,
            d: 10,
            m: 2,
            n: 50,
            o: 0,
            start_moment: None,
            b: 5,
            min_value: None,
            max_value: None,
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_logarithmic() {
        let original = DistributionFunction::Logarithmic {
            a: 100,
            d: 10,
            m: 2,
            n: 1,
            o: 1,
            start_moment: Some(0),
            b: 50,
            min_value: Some(10),
            max_value: Some(200),
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_logarithmic_none_options() {
        let original = DistributionFunction::Logarithmic {
            a: -5,
            d: 1,
            m: 1,
            n: 1,
            o: 0,
            start_moment: None,
            b: 100,
            min_value: None,
            max_value: None,
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_inverted_logarithmic() {
        let original = DistributionFunction::InvertedLogarithmic {
            a: 10000,
            d: 1,
            m: 1,
            n: 5000,
            o: 0,
            start_moment: Some(0),
            b: 0,
            min_value: Some(0),
            max_value: Some(100000),
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    #[test]
    fn round_trip_inverted_logarithmic_none_options() {
        let original = DistributionFunction::InvertedLogarithmic {
            a: -20,
            d: 5,
            m: 3,
            n: 10,
            o: -2,
            start_moment: None,
            b: 200,
            min_value: None,
            max_value: None,
        };
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    // -----------------------------------------------------------------------
    // Edge cases: zero values
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_fixed_amount_zero() {
        let original = DistributionFunction::FixedAmount { amount: 0 };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_random_zero_range() {
        let original = DistributionFunction::Random { min: 0, max: 0 };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_linear_all_zeros() {
        let original = DistributionFunction::Linear {
            a: 0,
            d: 0,
            start_step: Some(0),
            starting_amount: 0,
            min_value: Some(0),
            max_value: Some(0),
        };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_polynomial_all_zeros() {
        let original = DistributionFunction::Polynomial {
            a: 0,
            d: 0,
            m: 0,
            n: 0,
            o: 0,
            start_moment: Some(0),
            b: 0,
            min_value: Some(0),
            max_value: Some(0),
        };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_exponential_all_zeros() {
        let original = DistributionFunction::Exponential {
            a: 0,
            d: 0,
            m: 0,
            n: 0,
            o: 0,
            start_moment: Some(0),
            b: 0,
            min_value: Some(0),
            max_value: Some(0),
        };
        assert_eq!(round_trip(&original), original);
    }

    // -----------------------------------------------------------------------
    // Edge cases: max values
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_fixed_amount_max() {
        let original = DistributionFunction::FixedAmount { amount: u64::MAX };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_random_max_values() {
        let original = DistributionFunction::Random {
            min: u64::MAX - 1,
            max: u64::MAX,
        };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_step_decreasing_max_values() {
        let original = DistributionFunction::StepDecreasingAmount {
            step_count: u32::MAX,
            decrease_per_interval_numerator: u16::MAX,
            decrease_per_interval_denominator: u16::MAX,
            start_decreasing_offset: Some(u64::MAX),
            max_interval_count: Some(u16::MAX),
            distribution_start_amount: u64::MAX,
            trailing_distribution_interval_amount: u64::MAX,
            min_value: Some(u64::MAX),
        };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_linear_extreme_values() {
        let original = DistributionFunction::Linear {
            a: i64::MIN,
            d: u64::MAX,
            start_step: Some(u64::MAX),
            starting_amount: u64::MAX,
            min_value: Some(u64::MAX),
            max_value: Some(u64::MAX),
        };
        assert_eq!(round_trip(&original), original);

        let original2 = DistributionFunction::Linear {
            a: i64::MAX,
            d: 0,
            start_step: None,
            starting_amount: 0,
            min_value: None,
            max_value: None,
        };
        assert_eq!(round_trip(&original2), original2);
    }

    #[test]
    fn round_trip_polynomial_extreme_values() {
        let original = DistributionFunction::Polynomial {
            a: i64::MIN,
            d: u64::MAX,
            m: i64::MIN,
            n: u64::MAX,
            o: i64::MAX,
            start_moment: Some(u64::MAX),
            b: u64::MAX,
            min_value: Some(u64::MAX),
            max_value: Some(u64::MAX),
        };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_exponential_extreme_values() {
        let original = DistributionFunction::Exponential {
            a: u64::MAX,
            d: u64::MAX,
            m: i64::MIN,
            n: u64::MAX,
            o: i64::MIN,
            start_moment: Some(u64::MAX),
            b: u64::MAX,
            min_value: Some(u64::MAX),
            max_value: Some(u64::MAX),
        };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_logarithmic_extreme_values() {
        let original = DistributionFunction::Logarithmic {
            a: i64::MIN,
            d: u64::MAX,
            m: u64::MAX,
            n: u64::MAX,
            o: i64::MIN,
            start_moment: Some(u64::MAX),
            b: u64::MAX,
            min_value: Some(u64::MAX),
            max_value: Some(u64::MAX),
        };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_inverted_logarithmic_extreme_values() {
        let original = DistributionFunction::InvertedLogarithmic {
            a: i64::MAX,
            d: u64::MAX,
            m: u64::MAX,
            n: u64::MAX,
            o: i64::MAX,
            start_moment: Some(u64::MAX),
            b: u64::MAX,
            min_value: Some(u64::MAX),
            max_value: Some(u64::MAX),
        };
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_stepwise_single_entry() {
        let mut steps = BTreeMap::new();
        steps.insert(0, u64::MAX);
        let original = DistributionFunction::Stepwise(steps);
        assert_eq!(round_trip(&original), original);
    }

    #[test]
    fn round_trip_stepwise_many_entries() {
        let steps: BTreeMap<u64, u64> = (0..100).map(|i| (i * 10, i * 100 + 1)).collect();
        let original = DistributionFunction::Stepwise(steps);
        assert_eq!(round_trip(&original), original);
        assert_eq!(round_trip_borrow(&original), original);
    }

    // -----------------------------------------------------------------------
    // Determinism: same input always produces the same bytes
    // -----------------------------------------------------------------------

    #[test]
    fn encoding_is_deterministic() {
        let variants: Vec<DistributionFunction> = vec![
            DistributionFunction::FixedAmount { amount: 42 },
            DistributionFunction::Random { min: 1, max: 99 },
            DistributionFunction::StepDecreasingAmount {
                step_count: 100,
                decrease_per_interval_numerator: 1,
                decrease_per_interval_denominator: 2,
                start_decreasing_offset: Some(5),
                max_interval_count: Some(10),
                distribution_start_amount: 500,
                trailing_distribution_interval_amount: 1,
                min_value: Some(1),
            },
            DistributionFunction::Stepwise({
                let mut m = BTreeMap::new();
                m.insert(0, 100);
                m.insert(50, 50);
                m
            }),
            DistributionFunction::Linear {
                a: -2,
                d: 1,
                start_step: None,
                starting_amount: 100,
                min_value: None,
                max_value: Some(200),
            },
            DistributionFunction::Polynomial {
                a: 3,
                d: 1,
                m: 2,
                n: 1,
                o: 0,
                start_moment: None,
                b: 10,
                min_value: None,
                max_value: None,
            },
            DistributionFunction::Exponential {
                a: 100,
                d: 10,
                m: -3,
                n: 100,
                o: 0,
                start_moment: None,
                b: 10,
                min_value: None,
                max_value: None,
            },
            DistributionFunction::Logarithmic {
                a: 100,
                d: 10,
                m: 2,
                n: 1,
                o: 1,
                start_moment: None,
                b: 50,
                min_value: None,
                max_value: None,
            },
            DistributionFunction::InvertedLogarithmic {
                a: 10000,
                d: 1,
                m: 1,
                n: 5000,
                o: 0,
                start_moment: None,
                b: 0,
                min_value: None,
                max_value: None,
            },
        ];

        for variant in &variants {
            let bytes1 = bincode::encode_to_vec(variant, CONFIG).unwrap();
            let bytes2 = bincode::encode_to_vec(variant, CONFIG).unwrap();
            assert_eq!(
                bytes1, bytes2,
                "encoding was not deterministic for {:?}",
                variant
            );
        }
    }

    // -----------------------------------------------------------------------
    // Variant tag correctness: first byte encodes the variant discriminant
    // -----------------------------------------------------------------------

    #[test]
    fn variant_tags_are_correct() {
        let cases: Vec<(DistributionFunction, u8)> = vec![
            (DistributionFunction::FixedAmount { amount: 1 }, 0),
            (DistributionFunction::Random { min: 0, max: 1 }, 1),
            (
                DistributionFunction::StepDecreasingAmount {
                    step_count: 1,
                    decrease_per_interval_numerator: 1,
                    decrease_per_interval_denominator: 2,
                    start_decreasing_offset: None,
                    max_interval_count: None,
                    distribution_start_amount: 1,
                    trailing_distribution_interval_amount: 0,
                    min_value: None,
                },
                2,
            ),
            (DistributionFunction::Stepwise(BTreeMap::new()), 3),
            (
                DistributionFunction::Linear {
                    a: 0,
                    d: 1,
                    start_step: None,
                    starting_amount: 0,
                    min_value: None,
                    max_value: None,
                },
                4,
            ),
            (
                DistributionFunction::Polynomial {
                    a: 0,
                    d: 1,
                    m: 0,
                    n: 1,
                    o: 0,
                    start_moment: None,
                    b: 0,
                    min_value: None,
                    max_value: None,
                },
                5,
            ),
            (
                DistributionFunction::Exponential {
                    a: 0,
                    d: 1,
                    m: 0,
                    n: 1,
                    o: 0,
                    start_moment: None,
                    b: 0,
                    min_value: None,
                    max_value: None,
                },
                6,
            ),
            (
                DistributionFunction::Logarithmic {
                    a: 0,
                    d: 1,
                    m: 1,
                    n: 1,
                    o: 0,
                    start_moment: None,
                    b: 0,
                    min_value: None,
                    max_value: None,
                },
                7,
            ),
            (
                DistributionFunction::InvertedLogarithmic {
                    a: 0,
                    d: 1,
                    m: 1,
                    n: 1,
                    o: 0,
                    start_moment: None,
                    b: 0,
                    min_value: None,
                    max_value: None,
                },
                8,
            ),
        ];

        for (variant, expected_tag) in cases {
            let bytes = bincode::encode_to_vec(&variant, CONFIG).unwrap();
            assert_eq!(
                bytes[0], expected_tag,
                "wrong tag for {:?}: got {}, expected {}",
                variant, bytes[0], expected_tag
            );
        }
    }

    // -----------------------------------------------------------------------
    // Error paths: invalid variant tag
    // -----------------------------------------------------------------------

    #[test]
    fn decode_invalid_variant_tag_9() {
        let valid = DistributionFunction::FixedAmount { amount: 1 };
        let mut bytes = bincode::encode_to_vec(&valid, CONFIG).unwrap();
        bytes[0] = 9;
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(&bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_invalid_variant_tag_255() {
        let valid = DistributionFunction::FixedAmount { amount: 1 };
        let mut bytes = bincode::encode_to_vec(&valid, CONFIG).unwrap();
        bytes[0] = 255;
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(&bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn borrow_decode_invalid_variant_tag() {
        let valid = DistributionFunction::FixedAmount { amount: 1 };
        let mut bytes = bincode::encode_to_vec(&valid, CONFIG).unwrap();
        bytes[0] = 42;
        let result: Result<(DistributionFunction, _), _> =
            bincode::borrow_decode_from_slice(&bytes, CONFIG);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Error paths: truncated input
    // -----------------------------------------------------------------------

    #[test]
    fn decode_empty_input() {
        let bytes: &[u8] = &[];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_tag_only_fixed_amount() {
        let bytes: &[u8] = &[0];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_tag_only_random() {
        let bytes: &[u8] = &[1];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_tag_only_step_decreasing() {
        let bytes: &[u8] = &[2];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_tag_only_stepwise() {
        let bytes: &[u8] = &[3];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_tag_only_linear() {
        let bytes: &[u8] = &[4];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_tag_only_polynomial() {
        let bytes: &[u8] = &[5];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_tag_only_exponential() {
        let bytes: &[u8] = &[6];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_tag_only_logarithmic() {
        let bytes: &[u8] = &[7];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_tag_only_inverted_logarithmic() {
        let bytes: &[u8] = &[8];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_truncated_random_missing_max() {
        let original = DistributionFunction::Random { min: 10, max: 100 };
        let bytes = bincode::encode_to_vec(&original, CONFIG).unwrap();
        let truncated = &bytes[..bytes.len() / 2];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(truncated, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_truncated_linear_partial_payload() {
        let original = DistributionFunction::Linear {
            a: 5,
            d: 10,
            start_step: Some(100),
            starting_amount: 500,
            min_value: Some(1),
            max_value: Some(1000),
        };
        let bytes = bincode::encode_to_vec(&original, CONFIG).unwrap();
        let truncated = &bytes[..5];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(truncated, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_truncated_polynomial_partial_payload() {
        let original = DistributionFunction::Polynomial {
            a: 3,
            d: 1,
            m: 2,
            n: 1,
            o: -1,
            start_moment: Some(5),
            b: 100,
            min_value: Some(0),
            max_value: Some(10000),
        };
        let bytes = bincode::encode_to_vec(&original, CONFIG).unwrap();
        let truncated = &bytes[..bytes.len() - 3];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(truncated, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_truncated_exponential_partial_payload() {
        let original = DistributionFunction::Exponential {
            a: 100,
            d: 20,
            m: -3,
            n: 100,
            o: 5,
            start_moment: Some(10),
            b: 10,
            min_value: Some(1),
            max_value: Some(500),
        };
        let bytes = bincode::encode_to_vec(&original, CONFIG).unwrap();
        let truncated = &bytes[..bytes.len() - 5];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(truncated, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn decode_truncated_step_decreasing_partial_payload() {
        let original = DistributionFunction::StepDecreasingAmount {
            step_count: 210_000,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 2,
            start_decreasing_offset: Some(100),
            max_interval_count: Some(64),
            distribution_start_amount: 5000,
            trailing_distribution_interval_amount: 1,
            min_value: Some(10),
        };
        let bytes = bincode::encode_to_vec(&original, CONFIG).unwrap();
        let truncated = &bytes[..bytes.len() / 2];
        let result: Result<(DistributionFunction, _), _> =
            bincode::decode_from_slice(truncated, CONFIG);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Error paths: borrow_decode with truncated input
    // -----------------------------------------------------------------------

    #[test]
    fn borrow_decode_empty_input() {
        let bytes: &[u8] = &[];
        let result: Result<(DistributionFunction, _), _> =
            bincode::borrow_decode_from_slice(bytes, CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn borrow_decode_tag_only() {
        for tag in 0u8..=8 {
            let bytes: &[u8] = &[tag];
            let result: Result<(DistributionFunction, _), _> =
                bincode::borrow_decode_from_slice(bytes, CONFIG);
            assert!(
                result.is_err(),
                "borrow_decode should fail for tag-only input with tag {}",
                tag
            );
        }
    }

    #[test]
    fn borrow_decode_invalid_tag() {
        for tag in [9u8, 10, 50, 128, 255] {
            let bytes: &[u8] = &[tag];
            let result: Result<(DistributionFunction, _), _> =
                bincode::borrow_decode_from_slice(bytes, CONFIG);
            assert!(
                result.is_err(),
                "borrow_decode should fail for invalid tag {}",
                tag
            );
        }
    }

    // -----------------------------------------------------------------------
    // Decode and BorrowDecode produce the same results
    // -----------------------------------------------------------------------

    #[test]
    fn decode_and_borrow_decode_match_for_all_variants() {
        let variants: Vec<DistributionFunction> = vec![
            DistributionFunction::FixedAmount { amount: 777 },
            DistributionFunction::Random { min: 10, max: 1000 },
            DistributionFunction::StepDecreasingAmount {
                step_count: 500,
                decrease_per_interval_numerator: 3,
                decrease_per_interval_denominator: 100,
                start_decreasing_offset: Some(50),
                max_interval_count: Some(200),
                distribution_start_amount: 10000,
                trailing_distribution_interval_amount: 5,
                min_value: Some(1),
            },
            DistributionFunction::Stepwise({
                let mut m = BTreeMap::new();
                m.insert(0, 500);
                m.insert(100, 250);
                m.insert(200, 125);
                m
            }),
            DistributionFunction::Linear {
                a: -10,
                d: 3,
                start_step: Some(20),
                starting_amount: 1000,
                min_value: Some(100),
                max_value: None,
            },
            DistributionFunction::Polynomial {
                a: 5,
                d: 2,
                m: -1,
                n: 3,
                o: 7,
                start_moment: Some(10),
                b: 200,
                min_value: None,
                max_value: Some(5000),
            },
            DistributionFunction::Exponential {
                a: 250,
                d: 50,
                m: 1,
                n: 10,
                o: -3,
                start_moment: Some(5),
                b: 100,
                min_value: Some(50),
                max_value: Some(10000),
            },
            DistributionFunction::Logarithmic {
                a: 500,
                d: 20,
                m: 3,
                n: 2,
                o: -1,
                start_moment: Some(0),
                b: 75,
                min_value: Some(10),
                max_value: Some(1000),
            },
            DistributionFunction::InvertedLogarithmic {
                a: -100,
                d: 10,
                m: 5,
                n: 100,
                o: 2,
                start_moment: Some(3),
                b: 300,
                min_value: Some(0),
                max_value: Some(500),
            },
        ];

        for variant in &variants {
            let bytes = bincode::encode_to_vec(variant, CONFIG).unwrap();
            let (decoded, consumed1): (DistributionFunction, _) =
                bincode::decode_from_slice(&bytes, CONFIG).unwrap();
            let (borrow_decoded, consumed2): (DistributionFunction, _) =
                bincode::borrow_decode_from_slice(&bytes, CONFIG).unwrap();
            assert_eq!(
                decoded, borrow_decoded,
                "decode and borrow_decode differ for {:?}",
                variant
            );
            assert_eq!(
                consumed1, consumed2,
                "consumed bytes differ for {:?}",
                variant
            );
        }
    }

    // -----------------------------------------------------------------------
    // Negative i64 values round-trip correctly
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_negative_signed_fields() {
        let original = DistributionFunction::Polynomial {
            a: i64::MIN,
            d: 1,
            m: -8,
            n: 1,
            o: i64::MIN,
            start_moment: None,
            b: 0,
            min_value: None,
            max_value: None,
        };
        assert_eq!(round_trip(&original), original);

        let original2 = DistributionFunction::Exponential {
            a: 1,
            d: 1,
            m: i64::MIN,
            n: 1,
            o: i64::MIN,
            start_moment: None,
            b: 0,
            min_value: None,
            max_value: None,
        };
        assert_eq!(round_trip(&original2), original2);

        let original3 = DistributionFunction::InvertedLogarithmic {
            a: i64::MIN,
            d: 1,
            m: 1,
            n: 1,
            o: i64::MIN,
            start_moment: None,
            b: 0,
            min_value: None,
            max_value: None,
        };
        assert_eq!(round_trip(&original3), original3);
    }

    // -----------------------------------------------------------------------
    // Corrupted payload bytes
    // -----------------------------------------------------------------------

    #[test]
    fn decode_corrupted_option_byte_does_not_panic() {
        let original = DistributionFunction::Linear {
            a: 1,
            d: 1,
            start_step: None,
            starting_amount: 10,
            min_value: None,
            max_value: None,
        };
        let mut bytes = bincode::encode_to_vec(&original, CONFIG).unwrap();
        // Corrupt the last byte (an option discriminant for max_value)
        let last = bytes.len() - 1;
        bytes[last] = 5;
        // Should not panic regardless of outcome
        let _ = bincode::decode_from_slice::<DistributionFunction, _>(&bytes, CONFIG);
    }

    // -----------------------------------------------------------------------
    // Encode length varies correctly between variants
    // -----------------------------------------------------------------------

    #[test]
    fn fixed_amount_is_shortest_encoding() {
        let fixed = DistributionFunction::FixedAmount { amount: 1 };
        let random = DistributionFunction::Random { min: 1, max: 1 };
        let fixed_bytes = bincode::encode_to_vec(&fixed, CONFIG).unwrap();
        let random_bytes = bincode::encode_to_vec(&random, CONFIG).unwrap();
        assert!(
            fixed_bytes.len() <= random_bytes.len(),
            "FixedAmount should be shorter than or equal to Random"
        );
    }

    // -----------------------------------------------------------------------
    // Full round-trip: encode -> decode -> re-encode produces identical bytes
    // -----------------------------------------------------------------------

    #[test]
    fn double_round_trip_produces_identical_bytes() {
        let original = DistributionFunction::StepDecreasingAmount {
            step_count: 210_000,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 2,
            start_decreasing_offset: Some(100),
            max_interval_count: Some(64),
            distribution_start_amount: 5000,
            trailing_distribution_interval_amount: 1,
            min_value: Some(10),
        };
        let bytes1 = bincode::encode_to_vec(&original, CONFIG).unwrap();
        let (decoded, _): (DistributionFunction, _) =
            bincode::decode_from_slice(&bytes1, CONFIG).unwrap();
        let bytes2 = bincode::encode_to_vec(&decoded, CONFIG).unwrap();
        assert_eq!(bytes1, bytes2);
    }
}
