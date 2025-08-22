use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Modification of group action main parameters is not permitted.\n\
     Changed fields: {changed_internal_fields:?}\n\
     Original event type: {original}\n\
     Modified event type: {modified}"
)]
#[platform_serialize(unversioned)]
pub struct ModificationOfGroupActionMainParametersNotPermittedError {
    original: String,
    modified: String,
    changed_internal_fields: Vec<String>,
}

impl ModificationOfGroupActionMainParametersNotPermittedError {
    /// Creates a new `ModificationOfGroupActionMainParametersNotPermittedError`.
    pub fn new(original: String, modified: String, changed_internal_fields: Vec<String>) -> Self {
        Self {
            original,
            modified,
            changed_internal_fields,
        }
    }

    /// Returns a reference to the original group action.
    pub fn original(&self) -> &String {
        &self.original
    }

    /// Returns a reference to the modified group action.
    pub fn modified(&self) -> &String {
        &self.modified
    }

    /// Returns a reference to the changed fields.
    pub fn changed_internal_fields(&self) -> &Vec<String> {
        &self.changed_internal_fields
    }
}

impl From<ModificationOfGroupActionMainParametersNotPermittedError> for ConsensusError {
    fn from(err: ModificationOfGroupActionMainParametersNotPermittedError) -> Self {
        ConsensusError::StateError(
            StateError::ModificationOfGroupActionMainParametersNotPermittedError(err),
        )
    }
}
