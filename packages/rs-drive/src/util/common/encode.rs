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
