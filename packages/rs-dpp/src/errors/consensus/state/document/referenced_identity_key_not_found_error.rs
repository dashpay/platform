use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("referenced public key {key_id} of identity {identity_id} not found for path {path}")]
#[platform_serialize(unversioned)]
pub struct ReferencedIdentityKeyNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,
    key_id: KeyID,
    path: String,
}

impl ReferencedIdentityKeyNotFoundError {
    pub fn new(identity_id: Identifier, key_id: KeyID, path: String) -> Self {
        Self {
            identity_id,
            key_id,
            path,
        }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn key_id(&self) -> KeyID {
        self.key_id
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl From<ReferencedIdentityKeyNotFoundError> for ConsensusError {
    fn from(err: ReferencedIdentityKeyNotFoundError) -> Self {
        Self::StateError(StateError::ReferencedIdentityKeyNotFoundError(err))
    }
}
