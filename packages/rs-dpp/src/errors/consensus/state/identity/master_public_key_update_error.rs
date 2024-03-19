use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
)]
#[error("Failed to update the master public key: {description}. Ensure the operation is valid and permissible under current system rules.")]
#[platform_serialize(unversioned)]
pub struct MasterPublicKeyUpdateError {
    adding: usize,
    removing: usize,
    description: String,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MasterPublicKeyUpdateError {
    pub fn new(adding: usize, removing: usize) -> Self {
        let description = match (adding, removing) {
            (1, _) => "Attempt to add a new master key is not allowed unless one is being disabled"
                .to_string(),
            (0, _) => "Removing a master key without adding one is not allowed".to_string(),
            (_, 1) | (_, 0) => "Attempt to add more than one master key is not allowed".to_string(),
            (adding, removing) => format!(
                "Attempting to add {adding} master keys while removing {removing} master keys"
            ),
        };

        Self {
            adding,
            removing,
            description,
        }
    }

    pub fn adding(&self) -> usize {
        self.adding
    }

    pub fn removing(&self) -> usize {
        self.removing
    }
}
impl From<MasterPublicKeyUpdateError> for ConsensusError {
    fn from(err: MasterPublicKeyUpdateError) -> Self {
        Self::BasicError(BasicError::MasterPublicKeyUpdateError(err))
    }
}
