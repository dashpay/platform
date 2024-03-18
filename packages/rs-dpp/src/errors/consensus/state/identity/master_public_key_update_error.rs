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
    adding: bool,
    removing: bool,
    description: String,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MasterPublicKeyUpdateError {
    pub fn new(adding: bool, removing: bool) -> Self {
        let description = match (adding, removing) {
            (true, false) => "Attempt to add a new key is not allowed",
            (false, true) => "Attempt to remove the existing key is not allowed",
            (true, true) => "Simultaneous addition and removal of keys is not supported",
            (false, false) => "No operation specified",
        }
        .to_string();

        Self {
            adding,
            removing,
            description,
        }
    }

    pub fn is_adding(&self) -> bool {
        self.adding
    }

    pub fn is_removing(&self) -> bool {
        self.removing
    }
}
impl From<MasterPublicKeyUpdateError> for ConsensusError {
    fn from(err: MasterPublicKeyUpdateError) -> Self {
        Self::BasicError(BasicError::MasterPublicKeyUpdateError(err))
    }
}
