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
#[error("Identity Public Key with Id {id} does not exist")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityPublicKeyIdError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    id: KeyID,
}

impl InvalidIdentityPublicKeyIdError {
    pub fn new(id: KeyID) -> Self {
        Self { id }
    }

    pub fn id(&self) -> KeyID {
        self.id
    }
}
impl From<InvalidIdentityPublicKeyIdError> for ConsensusError {
    fn from(err: InvalidIdentityPublicKeyIdError) -> Self {
        Self::StateError(StateError::InvalidIdentityPublicKeyIdError(err))
    }
}
