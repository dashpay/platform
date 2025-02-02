use crate::balances::credits::{SignedTokenAmount, TokenAmount};
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use bincode::{BorrowDecode, Decode, Encode};
use ordered_float::NotNan;

/// Helper function to decode a `NotNan<f64>` safely.
fn decode_not_nan<D: bincode::de::Decoder>(decoder: &mut D) -> Result<NotNan<f64>, bincode::error::DecodeError> {
    NotNan::new(f64::decode(decoder)?).map_err(|_| {
        bincode::error::DecodeError::OtherString("Invalid float: NaN".into())
    })
}

// Implement Encode for DistributionFunction
impl Encode for DistributionFunction {
    fn encode<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        match self {
            DistributionFunction::FixedAmount { n } => {
                0u8.encode(encoder)?;
                n.encode(encoder)?;
            }
            DistributionFunction::StepDecreasingAmount { step_count, decrease_per_interval, n } => {
                1u8.encode(encoder)?;
                step_count.encode(encoder)?;
                decrease_per_interval.into_inner().encode(encoder)?;
                n.encode(encoder)?;
            }
            DistributionFunction::LinearInteger { a, b } => {
                2u8.encode(encoder)?;
                a.encode(encoder)?;
                b.encode(encoder)?;
            }
            DistributionFunction::LinearFloat { a, b } => {
                3u8.encode(encoder)?;
                a.into_inner().encode(encoder)?;
                b.encode(encoder)?;
            }
            DistributionFunction::PolynomialInteger { a, n, b } => {
                4u8.encode(encoder)?;
                a.encode(encoder)?;
                n.encode(encoder)?;
                b.encode(encoder)?;
            }
            DistributionFunction::PolynomialFloat { a, n, b } => {
                5u8.encode(encoder)?;
                a.into_inner().encode(encoder)?;
                n.into_inner().encode(encoder)?;
                b.encode(encoder)?;
            }
            DistributionFunction::Exponential { a, b, c } => {
                6u8.encode(encoder)?;
                a.into_inner().encode(encoder)?;
                b.into_inner().encode(encoder)?;
                c.encode(encoder)?;
            }
            DistributionFunction::Logarithmic { a, b, c } => {
                7u8.encode(encoder)?;
                a.into_inner().encode(encoder)?;
                b.into_inner().encode(encoder)?;
                c.encode(encoder)?;
            }
            DistributionFunction::Stepwise(steps) => {
                8u8.encode(encoder)?;
                steps.encode(encoder)?;
            }
        }
        Ok(())
    }
}

// Implement Decode for DistributionFunction
impl Decode for DistributionFunction {
    fn decode<D: bincode::de::Decoder>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError> {
        let variant = u8::decode(decoder)?;
        match variant {
            0 => {
                let n = TokenAmount::decode(decoder)?;
                Ok(Self::FixedAmount { n })
            }
            1 => {
                let step_count = u64::decode(decoder)?;
                let decrease_per_interval = decode_not_nan(decoder)?;
                let n = TokenAmount::decode(decoder)?;
                Ok(Self::StepDecreasingAmount { step_count, decrease_per_interval, n })
            }
            2 => {
                let a = i64::decode(decoder)?;
                let b = SignedTokenAmount::decode(decoder)?;
                Ok(Self::LinearInteger { a, b })
            }
            3 => {
                let a = decode_not_nan(decoder)?;
                let b = SignedTokenAmount::decode(decoder)?;
                Ok(Self::LinearFloat { a, b })
            }
            4 => {
                let a = i64::decode(decoder)?;
                let n = i64::decode(decoder)?;
                let b = SignedTokenAmount::decode(decoder)?;
                Ok(Self::PolynomialInteger { a, n, b })
            }
            5 => {
                let a = decode_not_nan(decoder)?;
                let n = decode_not_nan(decoder)?;
                let b = SignedTokenAmount::decode(decoder)?;
                Ok(Self::PolynomialFloat { a, n, b })
            }
            6 => {
                let a = decode_not_nan(decoder)?;
                let b = decode_not_nan(decoder)?;
                let c = SignedTokenAmount::decode(decoder)?;
                Ok(Self::Exponential { a, b, c })
            }
            7 => {
                let a = decode_not_nan(decoder)?;
                let b = decode_not_nan(decoder)?;
                let c = SignedTokenAmount::decode(decoder)?;
                Ok(Self::Logarithmic { a, b, c })
            }
            8 => {
                let steps = Vec::<(u64, TokenAmount)>::decode(decoder)?;
                Ok(Self::Stepwise(steps))
            }
            _ => Err(bincode::error::DecodeError::OtherString("Invalid variant".into())),
        }
    }
}

// Implement BorrowDecode for DistributionFunction
impl<'de> BorrowDecode<'de> for DistributionFunction {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError> {
        let variant = u8::borrow_decode(decoder)?;
        match variant {
            0 => {
                let n = TokenAmount::borrow_decode(decoder)?;
                Ok(Self::FixedAmount { n })
            }
            1 => {
                let step_count = u64::borrow_decode(decoder)?;
                let decrease_per_interval = decode_not_nan(decoder)?;
                let n = TokenAmount::borrow_decode(decoder)?;
                Ok(Self::StepDecreasingAmount { step_count, decrease_per_interval, n })
            }
            2 => {
                let a = i64::borrow_decode(decoder)?;
                let b = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(Self::LinearInteger { a, b })
            }
            3 => {
                let a = decode_not_nan(decoder)?;
                let b = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(Self::LinearFloat { a, b })
            }
            4 => {
                let a = i64::borrow_decode(decoder)?;
                let n = i64::borrow_decode(decoder)?;
                let b = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(Self::PolynomialInteger { a, n, b })
            }
            5 => {
                let a = decode_not_nan(decoder)?;
                let n = decode_not_nan(decoder)?;
                let b = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(Self::PolynomialFloat { a, n, b })
            }
            6 => {
                let a = decode_not_nan(decoder)?;
                let b = decode_not_nan(decoder)?;
                let c = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(Self::Exponential { a, b, c })
            }
            7 => {
                let a = decode_not_nan(decoder)?;
                let b = decode_not_nan(decoder)?;
                let c = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(Self::Logarithmic { a, b, c })
            }
            8 => {
                let steps = Vec::<(u64, TokenAmount)>::borrow_decode(decoder)?;
                Ok(Self::Stepwise(steps))
            }
            _ => Err(bincode::error::DecodeError::OtherString("Invalid variant".into())),
        }
    }
}