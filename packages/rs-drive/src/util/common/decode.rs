//! Decoding.
//!
//! This module defines decoding functions.
//!

use byteorder::{BigEndian, ReadBytesExt};
use std::io;

/// Decoding error.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// Slice passed to decode_u64 is not 8 bytes long.
    #[error("can't create a u64 from &[u8]: expected size 8, got {0}")]
    InvalidLength(usize),
    /// Unexpected IO error.
    #[error("can't create a u64 from &[u8]: expected size 8, got {0}")]
    ReadFailed(io::Error),
}

/// Decodes an unsigned integer on 64 bits.
pub fn decode_u64(val: &[u8]) -> Result<u64, DecodeError> {
    if val.len() != 8 {
        return Err(DecodeError::InvalidLength(val.len()));
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
    rdr.read_u64::<BigEndian>().map_err(DecodeError::ReadFailed)
}
