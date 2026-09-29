//! `sys.hash.*`: hash functions of one or more parameters, whose bytes they
//! join in order before hashing. A `refersTo` lookup key computes one over
//! values the referring document reveals, to find a commitment made earlier.

use crate::util::hash::hash_double;
use bincode::{Decode, DecodeUntrusted, Encode};

/// A hash function under `sys.hash`.
// @append_only
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub enum HashFunction {
    /// `sys.hash.sha256d`: SHA-256 of SHA-256, 32 bytes
    /// (`dpp::util::hash::hash_double`), the hash of a DPNS preorder.
    Sha256d,
}

impl HashFunction {
    /// Every hash function, in wire-name order.
    pub const ALL: [HashFunction; 1] = [HashFunction::Sha256d];

    /// The wire name, as the schema spells it.
    pub const fn as_str(&self) -> &'static str {
        match self {
            HashFunction::Sha256d => "sys.hash.sha256d",
        }
    }

    /// How many bytes the hash produces, the size of the byte array it fills.
    pub const fn output_length(&self) -> u16 {
        match self {
            HashFunction::Sha256d => 32,
        }
    }

    /// How many 64-byte blocks the hash compresses for a preimage of
    /// `preimage_len` bytes, what a write computing it is billed for: for
    /// `sys.hash.sha256d`, the padded preimage (the `0x80` byte and the 8-byte
    /// length SHA-256 appends) and one block for the second pass over the
    /// 32-byte digest. Saturates at `u16::MAX`, far past any preimage a
    /// document can hold.
    pub fn block_count(&self, preimage_len: usize) -> u16 {
        match self {
            HashFunction::Sha256d => {
                let first_pass_blocks = preimage_len.saturating_add(9).div_ceil(64);
                u16::try_from(first_pass_blocks.saturating_add(1)).unwrap_or(u16::MAX)
            }
        }
    }

    /// The hash of `preimage`, the parameters' bytes joined in order.
    pub fn digest(&self, preimage: &[u8]) -> [u8; 32] {
        match self {
            HashFunction::Sha256d => hash_double(preimage),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_count_the_padded_first_pass_and_the_second_pass() {
        // Up to 55 bytes pad to one block; 56 need a second; the second pass
        // over the 32-byte digest is one block more
        for (preimage_len, blocks) in [(0, 2), (42, 2), (55, 2), (56, 3), (119, 3), (120, 4)] {
            assert_eq!(
                HashFunction::Sha256d.block_count(preimage_len),
                blocks,
                "{preimage_len} bytes"
            );
        }
        assert_eq!(HashFunction::Sha256d.block_count(usize::MAX), u16::MAX);
    }
}
