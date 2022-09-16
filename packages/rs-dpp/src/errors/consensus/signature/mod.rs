use thiserror::Error;

use crate::{identity::KeyType, prelude::Identifier};

#[derive(Error, Debug)]
pub enum SignatureError {
    #[error("Public key {public_key_id} doesn't exist")]
    MissingPublicKeyError { public_key_id: u64 },

    #[error("Invalid identity public key typ {key_type}")]
    InvalidIdentityPublicKeyTypeError { key_type: KeyType },

    #[error("Invalid State Transition signature")]
    InvalidStateTransitionSignatureError,

    #[error("Identity {identity_id} not found")]
    IdentityNotFoundError { identity_id: Identifier },
}
