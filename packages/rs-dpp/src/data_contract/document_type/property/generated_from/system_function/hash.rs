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

    /// The hash of `preimage`, the parameters' bytes joined in order.
    pub fn digest(&self, preimage: &[u8]) -> [u8; 32] {
        match self {
            HashFunction::Sha256d => hash_double(preimage),
        }
    }
}
