use crate::consensus::signature::{IdentityNotFoundError, InvalidIdentityPublicKeyTypeError, InvalidSignaturePublicKeySecurityLevelError, InvalidStateTransitionSignatureError, MissingPublicKeyError, PublicKeyIsDisabledError, PublicKeySecurityLevelNotMetError, WrongPublicKeyPurposeError};
use crate::consensus::ConsensusError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SignatureError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[error(transparent)]
    IdentityNotFoundError(IdentityNotFoundError),

    #[error(transparent)]
    InvalidIdentityPublicKeyTypeError(InvalidIdentityPublicKeyTypeError),

    #[error(transparent)]
    InvalidStateTransitionSignatureError(InvalidStateTransitionSignatureError),

    #[error(transparent)]
    MissingPublicKeyError(MissingPublicKeyError),

    #[error(transparent)]
    InvalidSignaturePublicKeySecurityLevelError(InvalidSignaturePublicKeySecurityLevelError),

    #[error(transparent)]
    WrongPublicKeyPurposeError(WrongPublicKeyPurposeError),

    #[error(transparent)]
    PublicKeyIsDisabledError(PublicKeyIsDisabledError),

    #[error(transparent)]
    PublicKeySecurityLevelNotMetError(PublicKeySecurityLevelNotMetError),
}

impl From<SignatureError> for ConsensusError {
    fn from(err: SignatureError) -> Self {
        Self::SignatureError(err)
    }
}
