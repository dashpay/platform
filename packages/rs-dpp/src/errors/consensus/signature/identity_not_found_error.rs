use thiserror::Error;

use crate::errors::consensus::signature::signature_error::SignatureError;
use crate::errors::consensus::ConsensusError;
use platform_value::Identifier;

use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity {identity_id} not found")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct IdentityNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub identity_id: Identifier,
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
