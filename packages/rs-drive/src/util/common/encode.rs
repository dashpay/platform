#![allow(clippy::result_large_err)] // Encoding helpers bubble up drive::Error with context
//! Encoding.
//!
//! This module defines encoding functions.
//!

use crate::error::drive::DriveError;
use crate::error::Error;
use byteorder::{BigEndian, ByteOrder, WriteBytesExt};

/// Encodes an unsigned integer on 64 bits.
pub fn encode_u64(val: u64) -> Vec<u8> {
    // Positive integers are represented in binary with the signed bit set to 0
    // Negative integers are represented in 2's complement form

    // Encode the integer in big endian form
    // This ensures that most significant bits are compared first
    // a bigger positive number would be greater than a smaller one
    // and a bigger negative number would be greater than a smaller one
    // maintains sort order for each domain
    let mut wtr = vec![];
    wtr.write_u64::<BigEndian>(val).unwrap();

    // Flip the sign bit
    // to deal with interaction between the domains
    // 2's complement values have the sign bit set to 1
    // this makes them greater than the positive domain in terms of sort order
    // to fix this, we just flip the sign bit
    // so positive integers have the high bit and negative integers have the low bit
    // the relative order of elements in each domain is still maintained, as the
    // change was uniform across all elements
    wtr[0] ^= 0b1000_0000;

    wtr
}

/// Decodes a 64-bit unsigned integer from a vector of bytes encoded with `encode_u64`.
///
/// # Arguments
///
/// * `bytes` - A vector of bytes representing the encoded 64-bit unsigned integer.
///
/// # Returns
///
/// * A 64-bit unsigned integer decoded from the input bytes.
///
/// # Panics
///
/// This function will panic if the input vector does not have exactly 8 bytes.
pub fn decode_u64_owned(mut bytes: Vec<u8>) -> Result<u64, Error> {
    // Ensure the input vector has exactly 8 bytes
    if bytes.len() != 8 {
        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
            "Trying to decode a u64 from {} bytes {}",
            bytes.len(),
            hex::encode(bytes)
        ))));
    }

    // Flip the sign bit back to its original state
    // This reverses the transformation done in `encode_u64`
    bytes[0] ^= 0b1000_0000;

    // Read the integer from the modified bytes
    // The bytes are in big endian form, which preserves the correct order
    // when they were written in the encode function
    Ok(BigEndian::read_u64(&bytes))
}

/// Decodes a 64-bit unsigned integer from a vector of bytes encoded with `encode_u64`.
///
/// # Arguments
///
/// * `bytes` - A vector of bytes representing the encoded 64-bit unsigned integer.
///
/// # Returns
///
/// * A 64-bit unsigned integer decoded from the input bytes.
///
/// # Panics
///
/// This function will panic if the input vector does not have exactly 8 bytes.
pub fn decode_u64(bytes: &[u8]) -> Result<u64, Error> {
    // Ensure the input vector has exactly 8 bytes
    if bytes.len() != 8 {
        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
            "Trying to decode a u64 from {} bytes {}",
            bytes.len(),
            hex::encode(bytes)
        ))));
    }

    let mut wtr = bytes.to_vec();

    // Flip the sign bit back to its original state
    // This reverses the transformation done in `encode_u64`
    wtr[0] ^= 0b1000_0000;

    // Read the integer from the modified bytes
    // The bytes are in big endian form, which preserves the correct order
    // when they were written in the encode function
    Ok(BigEndian::read_u64(&wtr))
}

/// Encodes a signed integer on 64 bits.
pub fn encode_i64(val: i64) -> Vec<u8> {
    // Positive integers are represented in binary with the signed bit set to 0
    // Negative integers are represented in 2's complement form

    // Encode the integer in big endian form
    // This ensures that most significant bits are compared first
    // a bigger positive number would be greater than a smaller one
    // and a bigger negative number would be greater than a smaller one
    // maintains sort order for each domain
    let mut wtr = vec![];
    wtr.write_i64::<BigEndian>(val).unwrap();

    // Flip the sign bit
    // to deal with interaction between the domains
    // 2's complement values have the sign bit set to 1
    // this makes them greater than the positive domain in terms of sort order
    // to fix this, we just flip the sign bit
    // so positive integers have the high bit and negative integers have the low bit
    // the relative order of elements in each domain is still maintained, as the
    // change was uniform across all elements
    wtr[0] ^= 0b1000_0000;

    wtr
}

/// Encodes a float.
pub fn encode_float(val: f64) -> Vec<u8> {
    // Floats are represented based on the  IEEE 754-2008 standard
    // [sign bit] [biased exponent] [mantissa]

    // when comparing floats, the sign bit has the greatest impact
    // any positive number is greater than all negative numbers
    // if the numbers come from the same domain then the exponent is the next factor to consider
    // the exponent gives a sense of how many digits are in the non fractional part of the number
    // for example in base 10, 10 has an exponent of 1 (1.0 * 10^1)
    // while 5000 (5.0 * 10^3) has an exponent of 3
    // for the positive domain, the bigger the exponent the larger the number i.e 5000 > 10
    // for the negative domain, the bigger the exponent the smaller the number i.e -10 > -5000
    // if the exponents are the same, then the mantissa is used to determine the greater number
    // the inverse relationship still holds
    // i.e bigger mantissa (bigger number in positive domain but smaller number in negative domain)

    // There are two things to fix to achieve total sort order
    // 1. Place positive domain above negative domain (i.e flip the sign bit)
    // 2. Exponent and mantissa for a smaller number like -5000 is greater than that of -10
    //    so bit level comparison would say -5000 is greater than -10
    //    we fix this by flipping the exponent and mantissa values, which has the effect of reversing
    //    the order (0000 [smallest] -> 1111 [largest])

    // Encode in big endian form, so most significant bits are compared first
    let mut wtr = vec![];
    wtr.write_f64::<BigEndian>(val).unwrap();

    // Check if the value is negative, if it is
    // flip all the bits i.e sign, exponent and mantissa
    if val < 0.0 {
        wtr = wtr.iter().map(|byte| !byte).collect();
    } else {
        // for positive values, just flip the sign bit
        wtr[0] ^= 0b1000_0000;
    }

    wtr
}

/// Encodes an unsigned integer on 16 bits.
pub fn encode_u16(val: u16) -> Vec<u8> {
    // Positive integers are represented in binary with the signed bit set to 0
    // Negative integers are represented in 2's complement form

    // Encode the integer in big endian form
    // This ensures that most significant bits are compared first
    // a bigger positive number would be greater than a smaller one
    // and a bigger negative number would be greater than a smaller one
    // maintains sort order for each domain
    let mut wtr = vec![];
    wtr.write_u16::<BigEndian>(val).unwrap();

    // Flip the sign bit
    // to deal with interaction between the domains
    // 2's complement values have the sign bit set to 1
    // this makes them greater than the positive domain in terms of sort order
    // to fix this, we just flip the sign bit
    // so positive integers have the high bit and negative integers have the low bit
    // the relative order of elements in each domain is still maintained, as the
    // change was uniform across all elements
    wtr[0] ^= 0b1000_0000;

    wtr
}

/// Encodes an unsigned integer on 32 bits.
pub fn encode_u32(val: u32) -> Vec<u8> {
    // Positive integers are represented in binary with the signed bit set to 0
    // Negative integers are represented in 2's complement form

    // Encode the integer in big endian form
    // This ensures that most significant bits are compared first
    // a bigger positive number would be greater than a smaller one
    // and a bigger negative number would be greater than a smaller one
    // maintains sort order for each domain
    let mut wtr = vec![];
    wtr.write_u32::<BigEndian>(val).unwrap();

    // Flip the sign bit
    // to deal with interaction between the domains
    // 2's complement values have the sign bit set to 1
    // this makes them greater than the positive domain in terms of sort order
    // to fix this, we just flip the sign bit
    // so positive integers have the high bit and negative integers have the low bit
    // the relative order of elements in each domain is still maintained, as the
    // change was uniform across all elements
    wtr[0] ^= 0b1000_0000;

    wtr
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;

    // --- encode_u64 / decode_u64 round-trip tests ---

    #[test]
    fn encode_decode_u64_zero() {
        let encoded = encode_u64(0);
        assert_eq!(encoded.len(), 8);
        let decoded = decode_u64(&encoded).unwrap();
        assert_eq!(decoded, 0);
    }

    #[test]
    fn encode_decode_u64_one() {
        let encoded = encode_u64(1);
        let decoded = decode_u64(&encoded).unwrap();
        assert_eq!(decoded, 1);
    }

    #[test]
    fn encode_decode_u64_max() {
        let encoded = encode_u64(u64::MAX);
        let decoded = decode_u64(&encoded).unwrap();
        assert_eq!(decoded, u64::MAX);
    }

    #[test]
    fn encode_decode_u64_owned_round_trip() {
        for val in [0u64, 1, 42, 1000, u64::MAX / 2, u64::MAX] {
            let encoded = encode_u64(val);
            let decoded = decode_u64_owned(encoded).unwrap();
            assert_eq!(decoded, val);
        }
    }

    #[test]
    fn encode_u64_preserves_sort_order_in_positive_range() {
        // The sign-bit flip means lexicographic ordering matches signed interpretation.
        // Values in 0..=i64::MAX sort correctly among themselves.
        let values = [0u64, 1, 2, 100, 1000, i64::MAX as u64];
        let encoded: Vec<Vec<u8>> = values.iter().map(|&v| encode_u64(v)).collect();
        for i in 0..encoded.len() - 1 {
            assert!(
                encoded[i] < encoded[i + 1],
                "Sort order violated: encode_u64({}) >= encode_u64({})",
                values[i],
                values[i + 1]
            );
        }
    }

    #[test]
    fn encode_u64_sign_bit_flip_makes_high_values_sort_lower() {
        // Values above i64::MAX have the sign bit set in big-endian, so the flip
        // clears it, making them sort below values in the 0..=i64::MAX range.
        // This is the intended behavior: the encoding treats u64 as if it were i64.
        let below_midpoint = encode_u64(100);
        let above_midpoint = encode_u64(u64::MAX);
        assert!(above_midpoint < below_midpoint);
    }

    #[test]
    fn decode_u64_wrong_length_returns_error() {
        assert!(decode_u64(&[]).is_err());
        assert!(decode_u64(&[0; 7]).is_err());
        assert!(decode_u64(&[0; 9]).is_err());
        assert!(decode_u64(&[0; 1]).is_err());
    }

    #[test]
    fn decode_u64_owned_wrong_length_returns_error() {
        assert!(decode_u64_owned(vec![]).is_err());
        assert!(decode_u64_owned(vec![0; 7]).is_err());
        assert!(decode_u64_owned(vec![0; 9]).is_err());
    }

    // --- encode_i64 tests ---

    #[test]
    fn encode_i64_positive() {
        let encoded = encode_i64(42);
        assert_eq!(encoded.len(), 8);
    }

    #[test]
    fn encode_i64_negative() {
        let encoded = encode_i64(-42);
        assert_eq!(encoded.len(), 8);
    }

    #[test]
    fn encode_i64_zero() {
        let encoded = encode_i64(0);
        assert_eq!(encoded.len(), 8);
    }

    #[test]
    fn encode_i64_preserves_sort_order() {
        let values = [i64::MIN, -1000, -1, 0, 1, 1000, i64::MAX];
        let encoded: Vec<Vec<u8>> = values.iter().map(|&v| encode_i64(v)).collect();
        for i in 0..encoded.len() - 1 {
            assert!(
                encoded[i] < encoded[i + 1],
                "Sort order violated: encode_i64({}) >= encode_i64({})",
                values[i],
                values[i + 1]
            );
        }
    }

    #[test]
    fn encode_i64_negative_less_than_positive() {
        let neg = encode_i64(-1);
        let pos = encode_i64(1);
        assert!(neg < pos);
    }

    // --- encode_float tests ---

    #[test]
    fn encode_float_positive() {
        let encoded = encode_float(3.14);
        assert_eq!(encoded.len(), 8);
    }

    #[test]
    fn encode_float_negative() {
        let encoded = encode_float(-3.14);
        assert_eq!(encoded.len(), 8);
    }

    #[test]
    fn encode_float_zero() {
        let encoded = encode_float(0.0);
        assert_eq!(encoded.len(), 8);
    }

    #[test]
    fn encode_float_preserves_sort_order() {
        let values = [-1000.0f64, -1.0, -0.001, 0.0, 0.001, 1.0, 1000.0];
        let encoded: Vec<Vec<u8>> = values.iter().map(|&v| encode_float(v)).collect();
        for i in 0..encoded.len() - 1 {
            assert!(
                encoded[i] < encoded[i + 1],
                "Sort order violated: encode_float({}) >= encode_float({})",
                values[i],
                values[i + 1]
            );
        }
    }

    #[test]
    fn encode_float_negative_less_than_positive() {
        let neg = encode_float(-0.5);
        let pos = encode_float(0.5);
        assert!(neg < pos);
    }

    // --- encode_u16 tests ---

    #[test]
    fn encode_u16_basic() {
        assert_eq!(encode_u16(0).len(), 2);
        assert_eq!(encode_u16(u16::MAX).len(), 2);
    }

    #[test]
    fn encode_u16_preserves_sort_order_in_positive_range() {
        // Values in 0..=i16::MAX sort correctly after sign-bit flip.
        let values = [0u16, 1, 100, 1000, i16::MAX as u16];
        let encoded: Vec<Vec<u8>> = values.iter().map(|&v| encode_u16(v)).collect();
        for i in 0..encoded.len() - 1 {
            assert!(
                encoded[i] < encoded[i + 1],
                "Sort order violated: encode_u16({}) >= encode_u16({})",
                values[i],
                values[i + 1]
            );
        }
    }

    #[test]
    fn encode_u16_sign_bit_flip_makes_high_values_sort_lower() {
        let below = encode_u16(100);
        let above = encode_u16(u16::MAX);
        assert!(above < below);
    }

    // --- encode_u32 tests ---

    #[test]
    fn encode_u32_basic() {
        assert_eq!(encode_u32(0).len(), 4);
        assert_eq!(encode_u32(u32::MAX).len(), 4);
    }

    #[test]
    fn encode_u32_preserves_sort_order_in_positive_range() {
        // Values in 0..=i32::MAX sort correctly after sign-bit flip.
        let values = [0u32, 1, 100, 10000, i32::MAX as u32];
        let encoded: Vec<Vec<u8>> = values.iter().map(|&v| encode_u32(v)).collect();
        for i in 0..encoded.len() - 1 {
            assert!(
                encoded[i] < encoded[i + 1],
                "Sort order violated: encode_u32({}) >= encode_u32({})",
                values[i],
                values[i + 1]
            );
        }
    }

    #[test]
    fn encode_u32_sign_bit_flip_makes_high_values_sort_lower() {
        let below = encode_u32(100);
        let above = encode_u32(u32::MAX);
        assert!(above < below);
    }
}
