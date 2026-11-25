//! Retired: Platform state structure 0 as first shipped, tag 0 of `PlatformStateForSaving`.
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

mod old_structures;

/// Has no values: nothing can write this format, and decoding it always fails.
#[derive(Clone, Debug)]
pub enum PlatformStateForSavingV0 {}

impl Encode for PlatformStateForSavingV0 {
    fn encode<E: Encoder>(&self, _encoder: &mut E) -> Result<(), EncodeError> {
        match *self {}
    }
}

impl<Context> Decode<Context> for PlatformStateForSavingV0 {
    fn decode<D: Decoder<Context = Context>>(_decoder: &mut D) -> Result<Self, DecodeError> {
        Err(unsupported_legacy_bls_storage_error())
    }
}

bincode::impl_borrow_decode!(PlatformStateForSavingV0);
