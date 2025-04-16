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

impl Decode for DistributionFunction {
    fn decode<D: bincode::de::Decoder>(
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

impl<'de> BorrowDecode<'de> for DistributionFunction {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
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
