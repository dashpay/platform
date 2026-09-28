use crate::consensus::basic::BasicError;
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
#[error("Identity public key {public_key_id} has a budget of 0 credits: a budgeted key must be able to spend something")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityPublicKeyBudgetError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
}

impl InvalidIdentityPublicKeyBudgetError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }
}

impl From<InvalidIdentityPublicKeyBudgetError> for ConsensusError {
    fn from(err: InvalidIdentityPublicKeyBudgetError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityPublicKeyBudgetError(err))
    }
}
