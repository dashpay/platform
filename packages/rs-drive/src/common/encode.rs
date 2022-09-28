// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Encoding.
//!
//! This module defines encoding functions.
//!

use crate::error::Error;
use byteorder::{BigEndian, WriteBytesExt};

/// Encodes an unsigned integer.
pub fn encode_unsigned_integer(val: u64) -> Result<Vec<u8>, Error> {
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

    Ok(wtr)
}

/// Encodes a signed integer.
pub fn encode_signed_integer(val: i64) -> Result<Vec<u8>, Error> {
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

    Ok(wtr)
}

/// Encodes a float.
pub fn encode_float(val: f64) -> Result<Vec<u8>, Error> {
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

    Ok(wtr)
}
