use crate::consensus::signature::{
    IdentityNotFoundError, InvalidIdentityPublicKeyTypeError,
    InvalidSignaturePublicKeySecurityLevelError, MissingPublicKeyError, PublicKeyIsDisabledError,
    PublicKeySecurityLevelNotMetError, WrongPublicKeyPurposeError,
};
use crate::consensus::ConsensusError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SignatureError {
    #[error(transparent)]
    MissingPublicKeyError(MissingPublicKeyError),

    #[error(transparent)]
    InvalidIdentityPublicKeyTypeError(InvalidIdentityPublicKeyTypeError),

    #[error("Invalid State Transition signature")]
    InvalidStateTransitionSignatureError,

    #[error(transparent)]
    IdentityNotFoundError(IdentityNotFoundError),

    #[error(transparent)]
    InvalidSignaturePublicKeySecurityLevelError(InvalidSignaturePublicKeySecurityLevelError),

    #[error(transparent)]
    PublicKeyIsDisabledError(PublicKeyIsDisabledError),

    #[error(transparent)]
    PublicKeySecurityLevelNotMetError(PublicKeySecurityLevelNotMetError),

    #[error(transparent)]
    WrongPublicKeyPurposeError(WrongPublicKeyPurposeError),
}

impl From<SignatureError> for ConsensusError {
    fn from(err: SignatureError) -> Self {
        Self::SignatureError(err)
    }
}
