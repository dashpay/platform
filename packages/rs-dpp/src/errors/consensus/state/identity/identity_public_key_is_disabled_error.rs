use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity Public Key #{public_key_index} is disabled")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyIsDisabledError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_index: KeyID,
}

impl IdentityPublicKeyIsDisabledError {
    pub fn new(public_key_index: KeyID) -> Self {
        Self { public_key_index }
    }

    pub fn public_key_index(&self) -> KeyID {
        self.public_key_index
    }
}
impl From<IdentityPublicKeyIsDisabledError> for ConsensusError {
    fn from(err: IdentityPublicKeyIsDisabledError) -> Self {
        Self::StateError(StateError::IdentityPublicKeyIsDisabledError(err))
    }
}
