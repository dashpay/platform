//! Credits
//!
//! Credits are Platform native token and used for micro payments
//! between identities, state transitions fees and masternode rewards
//!
//! Credits are minted on Platform by locking Dash on payment chain and
//! can be withdrawn back to the payment chain by burning them on Platform
//! and unlocking dash on the payment chain.
//!

use crate::ProtocolError;
use integer_encoding::VarInt;
use std::convert::TryFrom;

/// Credits type

pub type Credits = u64;

/// Signed Credits type is used for internal computations and total credits
/// balance verification

pub type SignedCredits = i64;

/// Maximum value of credits

pub const MAX_CREDITS: Credits = 9223372036854775807 as Credits; //i64 Max

pub const CREDITS_PER_DUFF: Credits = 1000;

/// Trait for signed and unsigned credits

pub trait Creditable {
    /// Convert unsigned credit to singed
    fn to_signed(&self) -> Result<SignedCredits, ProtocolError>;
    /// Convert singed credit to unsigned
    fn to_unsigned(&self) -> Credits;

    // TODO: Should we implement serialize / unserialize traits instead?

    /// Decode bytes to credits
    fn from_vec_bytes(vec: Vec<u8>) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Encode credits to bytes
    fn to_vec_bytes(&self) -> Vec<u8>;
}

impl Creditable for Credits {
    fn to_signed(&self) -> Result<SignedCredits, ProtocolError> {
        SignedCredits::try_from(*self)
            .map_err(|_| ProtocolError::Overflow("credits are too big to convert to signed value"))
    }

    fn to_unsigned(&self) -> Credits {
        *self
    }

    fn from_vec_bytes(vec: Vec<u8>) -> Result<Self, ProtocolError> {
        Self::decode_var(vec.as_slice()).map(|(n, _)| n).ok_or(
            ProtocolError::CorruptedSerialization(
                "pending refunds epoch index for must be u16".to_string(),
            ),
        )
    }

    fn to_vec_bytes(&self) -> Vec<u8> {
        self.encode_var_vec()
    }
}

impl Creditable for SignedCredits {
    fn to_signed(&self) -> Result<SignedCredits, ProtocolError> {
        Ok(*self)
    }

    fn to_unsigned(&self) -> Credits {
        self.unsigned_abs()
    }

    fn from_vec_bytes(vec: Vec<u8>) -> Result<Self, ProtocolError> {
        Self::decode_var(vec.as_slice()).map(|(n, _)| n).ok_or(
            ProtocolError::CorruptedSerialization(
                "pending refunds epoch index for must be u16".to_string(),
            ),
        )
    }

    fn to_vec_bytes(&self) -> Vec<u8> {
        self.encode_var_vec()
    }
}
