use crate::consensus::signature::{
    BasicBLSError, BasicECDSAError, IdentityNotFoundError, InvalidIdentityPublicKeyTypeError,
    InvalidSignaturePublicKeySecurityLevelError, InvalidStateTransitionSignatureError,
    MissingPublicKeyError, PublicKeyIsDisabledError, PublicKeySecurityLevelNotMetError,
    SignatureShouldNotBePresentError, WrongPublicKeyPurposeError,
};
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use thiserror::Error;

use crate::consensus::signature::invalid_signature_public_key_purpose_error::InvalidSignaturePublicKeyPurposeError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

#[derive(Error, Debug, Encode, Decode, PlatformSerialize, PlatformDeserialize, Clone)]
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
    InvalidSignaturePublicKeyPurposeError(InvalidSignaturePublicKeyPurposeError),

    #[error(transparent)]
    InvalidSignaturePublicKeySecurityLevelError(InvalidSignaturePublicKeySecurityLevelError),

    #[error(transparent)]
    WrongPublicKeyPurposeError(WrongPublicKeyPurposeError),

    #[error(transparent)]
    PublicKeyIsDisabledError(PublicKeyIsDisabledError),

    #[error(transparent)]
    PublicKeySecurityLevelNotMetError(PublicKeySecurityLevelNotMetError),

    #[error(transparent)]
    SignatureShouldNotBePresentError(SignatureShouldNotBePresentError),

    #[error(transparent)]
    BasicECDSAError(BasicECDSAError),

    #[error(transparent)]
    BasicBLSError(BasicBLSError),
}

impl From<SignatureError> for ConsensusError {
    fn from(err: SignatureError) -> Self {
        Self::SignatureError(err)
    }
}
