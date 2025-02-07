use std::collections::BTreeMap;
use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use bincode::{BorrowDecode, Decode, Encode};

impl Encode for DistributionFunction {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        match self {
            DistributionFunction::FixedAmount { n } => {
                0u8.encode(encoder)?;
                n.encode(encoder)?;
            }
            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                s,
                n,
                min_value,
            } => {
                1u8.encode(encoder)?;
                step_count.encode(encoder)?;
                decrease_per_interval_numerator.encode(encoder)?;
                decrease_per_interval_denominator.encode(encoder)?;
                s.encode(encoder)?;
                n.encode(encoder)?;
                min_value.encode(encoder)?;
            }
            DistributionFunction::Stepwise(steps) => {
                2u8.encode(encoder)?;
                steps.encode(encoder)?;
            }
            DistributionFunction::Linear {
                a,
                d,
                s,
                b,
                min_value,
                max_value,
            } => {
                3u8.encode(encoder)?;
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
                s,
                b,
                min_value,
                max_value,
            } => {
                4u8.encode(encoder)?;
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
            DistributionFunction::Exponential {
                a,
                d,
                m,
                n,
                o,
                s,
                c,
                min_value,
                max_value,
            } => {
                5u8.encode(encoder)?;
                a.encode(encoder)?;
                d.encode(encoder)?;
                m.encode(encoder)?;
                n.encode(encoder)?;
                o.encode(encoder)?;
                s.encode(encoder)?;
                c.encode(encoder)?;
                min_value.encode(encoder)?;
                max_value.encode(encoder)?;
            }
            DistributionFunction::Logarithmic {
                a,
                d,
                m,
                n,
                o,
                s,
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
                Ok(Self::FixedAmount { n })
            }
            1 => {
                let step_count = u32::decode(decoder)?;
                let decrease_per_interval_numerator = u16::decode(decoder)?;
                let decrease_per_interval_denominator = u16::decode(decoder)?;
                let s = Option::<u64>::decode(decoder)?;
                let n = TokenAmount::decode(decoder)?;
                let min_value = Option::<u64>::decode(decoder)?;
                Ok(Self::StepDecreasingAmount {
                    s,
                    decrease_per_interval_numerator,
                    decrease_per_interval_denominator,
                    step_count,
                    n,
                    min_value,
                })
            }
            2 => {
                let steps = BTreeMap::<u64, TokenAmount>::decode(decoder)?;
                Ok(Self::Stepwise(steps))
            }
            3 => {
                let a = i64::decode(decoder)?;
                let d = u64::decode(decoder)?;
                let s = Option::<u64>::decode(decoder)?;
                let b = TokenAmount::decode(decoder)?;
                let min_value = Option::<u64>::decode(decoder)?;
                let max_value = Option::<u64>::decode(decoder)?;
                Ok(Self::Linear {
                    a,
                    d,
                    s,
                    b,
                    min_value,
                    max_value,
                })
            }
            4 => {
                let a = i64::decode(decoder)?;
                let d = u64::decode(decoder)?;
                let m = u64::decode(decoder)?;
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
                    s,
                    b,
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
                let c = TokenAmount::decode(decoder)?;
                let min_value = Option::<u64>::decode(decoder)?;
                let max_value = Option::<u64>::decode(decoder)?;
                Ok(Self::Exponential {
                    a,
                    d,
                    m,
                    n,
                    o,
                    s,
                    c,
                    min_value,
                    max_value,
                })
            }
            6 => {
                let a = i64::decode(decoder)?;
                let d = u64::decode(decoder)?;
                let m = i64::decode(decoder)?;
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
                    s,
                    b,
                    min_value,
                    max_value,
                })
            }
            _ => Err(bincode::error::DecodeError::OtherString("Invalid variant".into())),
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
                Ok(Self::FixedAmount { n })
            }
            1 => {
                let step_count = u32::borrow_decode(decoder)?;
                let decrease_per_interval_numerator = u16::borrow_decode(decoder)?;
                let decrease_per_interval_denominator = u16::borrow_decode(decoder)?;
                let s = Option::<u64>::borrow_decode(decoder)?;
                let n = TokenAmount::borrow_decode(decoder)?;
                let min_value = Option::<u64>::borrow_decode(decoder)?;
                Ok(Self::StepDecreasingAmount {
                    step_count,
                    decrease_per_interval_numerator,
                    decrease_per_interval_denominator,
                    s,
                    n,
                    min_value,
                })
            }
            2 => {
                let steps = BTreeMap::<u64, TokenAmount>::borrow_decode(decoder)?;
                Ok(Self::Stepwise(steps))
            }
            3 => {
                let a = i64::borrow_decode(decoder)?;
                let d = u64::borrow_decode(decoder)?;
                let s = Option::<u64>::borrow_decode(decoder)?;
                let b = TokenAmount::borrow_decode(decoder)?;
                let min_value = Option::<u64>::borrow_decode(decoder)?;
                let max_value = Option::<u64>::borrow_decode(decoder)?;
                Ok(Self::Linear {
                    a,
                    d,
                    s,
                    b,
                    min_value,
                    max_value,
                })
            }
            4 => {
                let a = i64::borrow_decode(decoder)?;
                let d = u64::borrow_decode(decoder)?;
                let m = u64::borrow_decode(decoder)?;
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
                    s,
                    b,
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
                let c = TokenAmount::borrow_decode(decoder)?;
                let min_value = Option::<u64>::borrow_decode(decoder)?;
                let max_value = Option::<u64>::borrow_decode(decoder)?;
                Ok(Self::Exponential {
                    a,
                    d,
                    m,
                    n,
                    o,
                    s,
                    c,
                    min_value,
                    max_value,
                })
            }
            6 => {
                let a = i64::borrow_decode(decoder)?;
                let d = u64::borrow_decode(decoder)?;
                let m = i64::borrow_decode(decoder)?;
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
                    s,
                    b,
                    min_value,
                    max_value,
                })
            }
            _ => Err(bincode::error::DecodeError::OtherString("Invalid variant".into())),
        }
    }
}