use crate::balances::credits::{SignedTokenAmount, TokenAmount};
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use bincode::{BorrowDecode, Decode, Encode};
use ordered_float::NotNan;

// Implement Encode for DistributionFunction
impl Encode for DistributionFunction {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        match self {
            DistributionFunction::LinearInteger { a, b } => {
                0u8.encode(encoder)?; // Variant index for LinearInteger
                a.encode(encoder)?;
                b.encode(encoder)?;
            }
            DistributionFunction::LinearFloat { a, b } => {
                1u8.encode(encoder)?; // Variant index for LinearFloat
                a.into_inner().encode(encoder)?; // Encode the NotNan<f64> value as f64
                b.encode(encoder)?;
            }
            DistributionFunction::PolynomialInteger { a, n, b } => {
                2u8.encode(encoder)?; // Variant index for PolynomialInteger
                a.encode(encoder)?;
                n.encode(encoder)?;
                b.encode(encoder)?;
            }
            DistributionFunction::PolynomialFloat { a, n, b } => {
                3u8.encode(encoder)?; // Variant index for PolynomialFloat
                a.into_inner().encode(encoder)?;
                n.into_inner().encode(encoder)?;
                b.encode(encoder)?;
            }
            DistributionFunction::Exponential { a, b, c } => {
                4u8.encode(encoder)?; // Variant index for Exponential
                a.into_inner().encode(encoder)?;
                b.into_inner().encode(encoder)?;
                c.encode(encoder)?;
            }
            DistributionFunction::Logarithmic { a, b, c } => {
                5u8.encode(encoder)?; // Variant index for Logarithmic
                a.into_inner().encode(encoder)?;
                b.into_inner().encode(encoder)?;
                c.encode(encoder)?;
            }
            DistributionFunction::Stepwise(steps) => {
                6u8.encode(encoder)?; // Variant index for Stepwise
                steps.encode(encoder)?;
            }
        }
        Ok(())
    }
}

// Implement Decode for DistributionFunction
impl Decode for DistributionFunction {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let variant = u8::decode(decoder)?;
        match variant {
            0 => {
                let a = i64::decode(decoder)?;
                let b = SignedTokenAmount::decode(decoder)?;
                Ok(Self::LinearInteger { a, b })
            }
            1 => {
                let a = NotNan::new(f64::decode(decoder)?).map_err(|_| {
                    bincode::error::DecodeError::OtherString("Invalid float: NaN".into())
                })?;
                let b = SignedTokenAmount::decode(decoder)?;
                Ok(Self::LinearFloat { a, b })
            }
            2 => {
                let a = i64::decode(decoder)?;
                let n = i64::decode(decoder)?;
                let b = SignedTokenAmount::decode(decoder)?;
                Ok(Self::PolynomialInteger { a, n, b })
            }
            3 => {
                let a = NotNan::new(f64::decode(decoder)?).map_err(|_| {
                    bincode::error::DecodeError::OtherString("Invalid float: NaN".into())
                })?;
                let n = NotNan::new(f64::decode(decoder)?).map_err(|_| {
                    bincode::error::DecodeError::OtherString("Invalid float: NaN".into())
                })?;
                let b = SignedTokenAmount::decode(decoder)?;
                Ok(Self::PolynomialFloat { a, n, b })
            }
            4 => {
                let a = NotNan::new(f64::decode(decoder)?).map_err(|_| {
                    bincode::error::DecodeError::OtherString("Invalid float: NaN".into())
                })?;
                let b = NotNan::new(f64::decode(decoder)?).map_err(|_| {
                    bincode::error::DecodeError::OtherString("Invalid float: NaN".into())
                })?;
                let c = SignedTokenAmount::decode(decoder)?;
                Ok(Self::Exponential { a, b, c })
            }
            5 => {
                let a = NotNan::new(f64::decode(decoder)?).map_err(|_| {
                    bincode::error::DecodeError::OtherString("Invalid float: NaN".into())
                })?;
                let b = NotNan::new(f64::decode(decoder)?).map_err(|_| {
                    bincode::error::DecodeError::OtherString("Invalid float: NaN".into())
                })?;
                let c = SignedTokenAmount::decode(decoder)?;
                Ok(Self::Logarithmic { a, b, c })
            }
            6 => {
                let steps = Vec::<(u64, TokenAmount)>::decode(decoder)?;
                Ok(Self::Stepwise(steps))
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
        let variant = u8::borrow_decode(decoder)?; // Decode the variant tag

        match variant {
            0 => {
                let a = i64::borrow_decode(decoder)?;
                let b = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(DistributionFunction::LinearInteger { a, b })
            }
            1 => {
                let a = NotNan::<f64>::new(f64::borrow_decode(decoder)?)
                    .map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))?;
                let b = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(DistributionFunction::LinearFloat { a, b })
            }
            2 => {
                let a = i64::borrow_decode(decoder)?;
                let n = i64::borrow_decode(decoder)?;
                let b = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(DistributionFunction::PolynomialInteger { a, n, b })
            }
            3 => {
                let a = NotNan::<f64>::new(f64::borrow_decode(decoder)?)
                    .map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))?;
                let n = NotNan::<f64>::new(f64::borrow_decode(decoder)?)
                    .map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))?;
                let b = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(DistributionFunction::PolynomialFloat { a, n, b })
            }
            4 => {
                let a = NotNan::<f64>::new(f64::borrow_decode(decoder)?)
                    .map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))?;
                let b = NotNan::<f64>::new(f64::borrow_decode(decoder)?)
                    .map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))?;
                let c = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(DistributionFunction::Exponential { a, b, c })
            }
            5 => {
                let a = NotNan::<f64>::new(f64::borrow_decode(decoder)?)
                    .map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))?;
                let b = NotNan::<f64>::new(f64::borrow_decode(decoder)?)
                    .map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))?;
                let c = SignedTokenAmount::borrow_decode(decoder)?;
                Ok(DistributionFunction::Logarithmic { a, b, c })
            }
            6 => {
                let steps = Vec::<(u64, TokenAmount)>::borrow_decode(decoder)?;
                Ok(DistributionFunction::Stepwise(steps))
            }
            _ => Err(bincode::error::DecodeError::OtherString(
                "Invalid variant tag for DistributionFunction".to_string(),
            )),
        }
    }
}
