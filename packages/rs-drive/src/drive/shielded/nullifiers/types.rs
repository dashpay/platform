use crate::error::Error;
use dpp::ProtocolError;
use std::ops::Deref;

/// Wrapper around `Vec<[u8; 32]>` for serialized nullifier lists.
///
/// Encapsulates the bincode configuration used for encoding and decoding
/// compacted nullifier data, eliminating duplicated serialization boilerplate.
pub struct CompactedNullifiers(Vec<[u8; 32]>);

impl CompactedNullifiers {
    /// Creates a new `CompactedNullifiers` from the given nullifier list.
    pub fn new(inner: Vec<[u8; 32]>) -> Self {
        Self(inner)
    }

    /// Returns the bincode configuration used for nullifier serialization.
    fn bincode_config() -> bincode::config::Configuration<
        bincode::config::BigEndian,
        bincode::config::Varint,
        bincode::config::NoLimit,
    > {
        bincode::config::standard()
            .with_big_endian()
            .with_no_limit()
    }

    /// Encodes the nullifier list into bytes using bincode.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        bincode::encode_to_vec(&self.0, Self::bincode_config()).map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "cannot encode compacted nullifiers: {}",
                e
            ))))
        })
    }

    /// Decodes a nullifier list from bytes using bincode.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let (inner, _): (Vec<[u8; 32]>, usize) =
            bincode::decode_from_slice(bytes, Self::bincode_config()).map_err(|e| {
                Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                    "cannot decode compacted nullifiers: {}",
                    e
                ))))
            })?;
        Ok(Self(inner))
    }

    /// Consumes the wrapper and returns the inner `Vec<[u8; 32]>`.
    pub fn into_inner(self) -> Vec<[u8; 32]> {
        self.0
    }
}

impl Deref for CompactedNullifiers {
    type Target = Vec<[u8; 32]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Wrapper around `Vec<(u64, u64)>` for serialized nullifier expiration range lists.
///
/// Each `(u64, u64)` pair represents a `(start_block, end_block)` range of compacted
/// nullifiers that share the same expiration timestamp. Encapsulates the bincode
/// configuration used for encoding and decoding these ranges.
pub struct NullifierExpirationRanges(Vec<(u64, u64)>);

impl NullifierExpirationRanges {
    /// Creates a new `NullifierExpirationRanges` from the given range list.
    pub fn new(inner: Vec<(u64, u64)>) -> Self {
        Self(inner)
    }

    /// Returns the bincode configuration used for expiration range serialization.
    fn bincode_config() -> bincode::config::Configuration<
        bincode::config::BigEndian,
        bincode::config::Varint,
        bincode::config::NoLimit,
    > {
        bincode::config::standard()
            .with_big_endian()
            .with_no_limit()
    }

    /// Encodes the expiration range list into bytes using bincode.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        bincode::encode_to_vec(&self.0, Self::bincode_config()).map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "cannot encode nullifier expiration ranges: {}",
                e
            ))))
        })
    }

    /// Decodes an expiration range list from bytes using bincode.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let (inner, _): (Vec<(u64, u64)>, usize) =
            bincode::decode_from_slice(bytes, Self::bincode_config()).map_err(|e| {
                Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                    "cannot decode nullifier expiration ranges: {}",
                    e
                ))))
            })?;
        Ok(Self(inner))
    }

    /// Consumes the wrapper and returns the inner `Vec<(u64, u64)>`.
    pub fn into_inner(self) -> Vec<(u64, u64)> {
        self.0
    }
}

impl Deref for NullifierExpirationRanges {
    type Target = Vec<(u64, u64)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
