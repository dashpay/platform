use thiserror::Error;

use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::prelude::Identifier;

use serde::{Deserialize, Serialize};

use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Identity {identity_id} not found")]
pub struct IdentityNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,
}

impl IdentityNotFoundError {
    pub fn new(identity_id: Identifier) -> Self {
        Self { identity_id }
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}

impl From<IdentityNotFoundError> for ConsensusError {
    fn from(err: IdentityNotFoundError) -> Self {
        Self::SignatureError(SignatureError::IdentityNotFoundError(err))
    }
}
