use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::prelude::{Identifier, Revision};

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity {identity_id} has invalid revision. The current revision is {current_revision}")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityRevisionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,
    current_revision: Revision,
}

impl InvalidIdentityRevisionError {
    pub fn new(identity_id: Identifier, current_revision: Revision) -> Self {
        Self {
            identity_id,
            current_revision,
        }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
    pub fn current_revision(&self) -> &Revision {
        &self.current_revision
    }
}
impl From<InvalidIdentityRevisionError> for ConsensusError {
    fn from(err: InvalidIdentityRevisionError) -> Self {
        Self::StateError(StateError::InvalidIdentityRevisionError(err))
    }
}
