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

//! Decoding.
//!
//! This module defines decoding functions.
//!

use byteorder::{BigEndian, ReadBytesExt};

/// Decodes an unsigned integer on 64 bits.
pub fn decode_u64(val: &[u8]) -> Result<u64, ()> {
    if val.len() != 8 {
        return Err(());
    }

    // Flip the sign bit
    // to deal with interaction between the domains
    // 2's complement values have the sign bit set to 1
    // this makes them greater than the positive domain in terms of sort order
    // to fix this, we just flip the sign bit
    // so positive integers have the high bit and negative integers have the low bit
    // the relative order of elements in each domain is still maintained, as the
    // change was uniform across all elements
    let mut val = val.to_vec();
    val[0] ^= 0b1000_0000;

    // Decode the integer in big endian form
    // This ensures that most significant bits are compared first
    // a bigger positive number would be greater than a smaller one
    // and a bigger negative number would be greater than a smaller one
    // maintains sort order for each domain
    let mut rdr = val.as_slice();
    rdr.read_u64::<BigEndian>().map_err(|_| ())
}
