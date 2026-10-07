//! Rejection of database formats that require an intermediate Platform upgrade.

use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};

/// Uninhabited payload reserving a legacy enum tag without accepting its data.
///
/// Keeping the variant slot preserves the tags of supported storage formats.
#[derive(Clone, Debug)]
pub enum UnsupportedLegacyBlsStorage {}

impl Encode for UnsupportedLegacyBlsStorage {
    fn encode<E: Encoder>(&self, _encoder: &mut E) -> Result<(), EncodeError> {
        match *self {}
    }
}

impl<Context> Decode<Context> for UnsupportedLegacyBlsStorage {
    fn decode<D: Decoder<Context = Context>>(_decoder: &mut D) -> Result<Self, DecodeError> {
        Err(DecodeError::Other(
            "Unsupported legacy BLS state format. Run Platform v4.0.0 on this database and let it successfully commit at least one block before upgrading.",
        ))
    }
}

bincode::impl_borrow_decode!(UnsupportedLegacyBlsStorage);
