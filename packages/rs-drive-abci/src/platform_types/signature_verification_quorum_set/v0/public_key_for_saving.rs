use crate::platform_types::signature_verification_quorum_set::ThresholdBlsPublicKey;
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};

/// The disk contract is 48 compressed G1 bytes, independent of the crypto backend's Serde format.
#[derive(Debug, Clone)]
pub(super) struct PublicKeyForSaving(pub(super) ThresholdBlsPublicKey);

impl Encode for PublicKeyForSaving {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.0.to_bytes().encode(encoder)
    }
}

impl<C> Decode<C> for PublicKeyForSaving {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes = <[u8; 48]>::decode(decoder)?;
        ThresholdBlsPublicKey::try_from(bytes.as_slice())
            .map(Self)
            .map_err(|_| DecodeError::Other("invalid compressed quorum public key"))
    }
}

bincode::impl_borrow_decode!(PublicKeyForSaving);
