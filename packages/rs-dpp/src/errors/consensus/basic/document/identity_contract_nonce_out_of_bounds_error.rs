use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::prelude::IdentityNonce;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("Nonce is out of bounds: {}", nonce)]
#[platform_serialize(unversioned)]
pub struct NonceOutOfBoundsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    nonce: IdentityNonce,
}

impl NonceOutOfBoundsError {
    pub fn new(nonce: IdentityNonce) -> Self {
        Self { nonce }
    }

    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        self.nonce
    }
}

impl From<NonceOutOfBoundsError> for ConsensusError {
    fn from(err: NonceOutOfBoundsError) -> Self {
        Self::BasicError(BasicError::NonceOutOfBoundsError(err))
    }
}
