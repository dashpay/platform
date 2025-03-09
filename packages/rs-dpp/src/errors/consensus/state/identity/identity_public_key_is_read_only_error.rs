use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::identity_public_key::KeyID;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity Public Key #{public_key_index} is read only")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct IdentityPublicKeyIsReadOnlyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub public_key_index: KeyID,
}

impl IdentityPublicKeyIsReadOnlyError {
    pub fn new(public_key_index: KeyID) -> Self {
        Self { public_key_index }
    }

    pub fn public_key_index(&self) -> KeyID {
        self.public_key_index
    }
}
impl From<IdentityPublicKeyIsReadOnlyError> for ConsensusError {
    fn from(err: IdentityPublicKeyIsReadOnlyError) -> Self {
        Self::StateError(StateError::IdentityPublicKeyIsReadOnlyError(err))
    }
}
