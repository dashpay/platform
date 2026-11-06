use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// An identity update adds a HECATE_SIGNER key while the identity keeps another enabled: an
/// identity holds at most one, so the update must disable the old one to add a new one
/// (protocol version 15).
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
#[error("Identity {identity_id} already has the enabled HECATE_SIGNER key {existing_public_key_id}: an identity holds at most one, so disable it in the same update to add another")]
#[platform_serialize(unversioned)]
pub struct IdentityAlreadyHasHecateSignerKeyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,
    existing_public_key_id: KeyID,
}

impl IdentityAlreadyHasHecateSignerKeyError {
    pub fn new(identity_id: Identifier, existing_public_key_id: KeyID) -> Self {
        Self {
            identity_id,
            existing_public_key_id,
        }
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    pub fn existing_public_key_id(&self) -> KeyID {
        self.existing_public_key_id
    }
}

impl From<IdentityAlreadyHasHecateSignerKeyError> for ConsensusError {
    fn from(err: IdentityAlreadyHasHecateSignerKeyError) -> Self {
        Self::StateError(StateError::IdentityAlreadyHasHecateSignerKeyError(err))
    }
}
