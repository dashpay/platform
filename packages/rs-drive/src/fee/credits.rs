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

//! Credits
//!
//! Credits are Platform native token and used for micro payments
//! between identities, state transitions fees and masternode rewards
//!
//! Credits are minted on Platform by locking Dash on payment chain and
//! can be withdrawn back to the payment chain by burning them on Platform
//! and unlocking dash on the payment chain.
//!

// TODO: Should be moved to DPP when integration is done

use integer_encoding::VarInt;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::get_overflow_error;
use rust_decimal::Decimal;

/// Credits type
pub type Credits = u64;

/// Signed Credits type is used for internal computations and total credits
/// balance verification
pub type SignedCredits = i64;

/// Maximum value of credits
pub const MAX_CREDITS: Credits = SignedCredits::MAX as Credits;

/// Trait for signed and unsigned credits
pub trait Creditable: Into<Decimal> {
    /// Convert unsigned credit to singed
    fn to_signed(&self) -> Result<SignedCredits, Error>;
    /// Convert singed credit to unsigned
    fn to_unsigned(&self) -> Credits;

    // TODO: Should we implement serialize / unserialize traits instead?

    /// Decode bytes to credits
    fn from_vec_bytes(vec: Vec<u8>) -> Result<Self, Error>;
    /// Encode credits to bytes
    fn to_vec_bytes(&self) -> Vec<u8>;
}

impl Creditable for Credits {
    fn to_signed(&self) -> Result<SignedCredits, Error> {
        SignedCredits::try_from(*self)
            .map_err(|_| get_overflow_error("credits are too big to convert to signed value"))
    }

    fn to_unsigned(&self) -> Credits {
        *self
    }

    fn from_vec_bytes(vec: Vec<u8>) -> Result<Self, Error> {
        Self::decode_var(vec.as_slice()).map(|(n, s)| n).ok_or(
            Error::Drive(DriveError::CorruptedSerialization(
                "pending updates epoch index for must be u16",
            ))
        )
    }

    fn to_vec_bytes(&self) -> Vec<u8> {
        self.encode_var_vec()
    }
}
impl Creditable for SignedCredits {
    fn to_signed(&self) -> Result<SignedCredits, Error> {
        Ok(*self)
    }

    fn to_unsigned(&self) -> Credits {
        self.unsigned_abs()
    }

    fn from_vec_bytes(vec: Vec<u8>) -> Result<Self, Error> {
        Self::decode_var(vec.as_slice()).map(|(n, s)| n).ok_or(
            Error::Drive(DriveError::CorruptedSerialization(
                "pending updates epoch index for must be u16",
            ))
        )
    }

    fn to_vec_bytes(&self) -> Vec<u8> {
        self.encode_var_vec()
    }
}
