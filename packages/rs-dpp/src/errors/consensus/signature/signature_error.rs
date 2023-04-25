use crate::consensus::signature::{
    IdentityNotFoundError, InvalidIdentityPublicKeyTypeError,
    InvalidSignaturePublicKeySecurityLevelError, InvalidStateTransitionSignatureError,
    MissingPublicKeyError, PublicKeyIsDisabledError, PublicKeySecurityLevelNotMetError,
    WrongPublicKeyPurposeError,
};
use crate::consensus::ConsensusError;
use thiserror::Error;

use serde::{Deserialize, Serialize};

#[derive(Error, Debug, Serialize, Deserialize)]
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

    #[error("signature should be empty {0}")]
    SignatureShouldNotBePresent(String),

    #[error("ecdsa signing error {0}")]
    BasicECDSAError(String),

    #[error("bls signing error {0}")]
    BasicBLSError(String),
}

impl From<SignatureError> for ConsensusError {
    fn from(err: SignatureError) -> Self {
        Self::SignatureError(err)
    }
}
