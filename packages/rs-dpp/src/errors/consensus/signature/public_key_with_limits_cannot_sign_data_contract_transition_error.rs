use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
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
#[error("Identity public key {public_key_id} carries a budget or an expiry and can not create or update a data contract: only a key without limits can")]
#[platform_serialize(unversioned)]
pub struct PublicKeyWithLimitsCannotSignDataContractTransitionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
}

impl PublicKeyWithLimitsCannotSignDataContractTransitionError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }
}

impl From<PublicKeyWithLimitsCannotSignDataContractTransitionError> for ConsensusError {
    fn from(err: PublicKeyWithLimitsCannotSignDataContractTransitionError) -> Self {
        Self::SignatureError(
            SignatureError::PublicKeyWithLimitsCannotSignDataContractTransitionError(err),
        )
    }
}
