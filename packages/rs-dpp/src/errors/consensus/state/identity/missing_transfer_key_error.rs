use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity {identity_id} does not have a key for transferring funds")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct MissingTransferKeyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub identity_id: Identifier,
}

impl MissingTransferKeyError {
    pub fn new(identity_id: Identifier) -> Self {
        Self { identity_id }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}
impl From<MissingTransferKeyError> for ConsensusError {
    fn from(err: MissingTransferKeyError) -> Self {
        Self::StateError(StateError::MissingTransferKeyError(err))
    }
}
