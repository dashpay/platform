//! Retired: quorum storage version 0, tag 0 of `SignatureVerificationQuorumSetForSaving`.
//!
//! The reader was removed in Platform v5.1.0 by #5323, to drop the C++ `bls-signatures`
//! dependency from the default production node: the reader decoded this format's keys through
//! that crate. Platform v4.0.0 is the upgrade path, as it reads this format and writes a
//! supported one. Only this placeholder remains, and it keeps the tag reserved, so the tags of
//! the supported formats do not move.

use crate::platform_types::unsupported_legacy_bls_storage::unsupported_legacy_bls_storage_error;
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};

/// Has no values: nothing can write this format, and decoding it always fails.
#[derive(Debug, Clone)]
pub enum SignatureVerificationQuorumSetForSavingV0 {}

impl Encode for SignatureVerificationQuorumSetForSavingV0 {
    fn encode<E: Encoder>(&self, _encoder: &mut E) -> Result<(), EncodeError> {
        match *self {}
    }
}

impl<Context> Decode<Context> for SignatureVerificationQuorumSetForSavingV0 {
    fn decode<D: Decoder<Context = Context>>(_decoder: &mut D) -> Result<Self, DecodeError> {
        Err(unsupported_legacy_bls_storage_error())
    }
}

bincode::impl_borrow_decode!(SignatureVerificationQuorumSetForSavingV0);
