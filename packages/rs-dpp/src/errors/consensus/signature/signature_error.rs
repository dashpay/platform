use crate::consensus::signature::ContractBoundedKeyNonBatchError;
use crate::consensus::signature::ContractBoundedKeyOutOfBoundsError;
use crate::consensus::signature::PublicKeyBudgetExhaustedError;
use crate::consensus::signature::PublicKeyExpiredError;
use crate::consensus::signature::PublicKeyWithLimitsCannotUpdateKeyLimitsError;
use crate::consensus::signature::{
    BasicBLSError, BasicECDSAError, IdentityNotFoundError, InvalidIdentityPublicKeyTypeError,
    InvalidSignaturePublicKeySecurityLevelError, InvalidStateTransitionSignatureError,
    MissingPublicKeyError, PublicKeyIsDisabledError, PublicKeySecurityLevelNotMetError,
    SignatureShouldNotBePresentError, UncompressedPublicKeyNotAllowedError,
    WrongPublicKeyPurposeError,
};
use crate::consensus::ConsensusError;
use bincode::{Decode, DecodeUntrusted, Encode};
use thiserror::Error;

use crate::consensus::signature::invalid_signature_public_key_purpose_error::InvalidSignaturePublicKeyPurposeError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};

#[derive(
    Error,
    Debug,
    PartialEq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    Clone,
    DecodeUntrusted,
)]
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

    #[error(transparent)]
    UncompressedPublicKeyNotAllowedError(UncompressedPublicKeyNotAllowedError),
    #[error(transparent)]
    ContractBoundedKeyNonBatchError(ContractBoundedKeyNonBatchError),

    #[error(transparent)]
    ContractBoundedKeyOutOfBoundsError(ContractBoundedKeyOutOfBoundsError),

    // Authentication key limits (protocol version 14).
    #[error(transparent)]
    PublicKeyBudgetExhaustedError(PublicKeyBudgetExhaustedError),

    #[error(transparent)]
    PublicKeyExpiredError(PublicKeyExpiredError),

    // Identity key limits update (protocol version 14).
    #[error(transparent)]
    PublicKeyWithLimitsCannotUpdateKeyLimitsError(PublicKeyWithLimitsCannotUpdateKeyLimitsError),
}

impl From<SignatureError> for ConsensusError {
    fn from(err: SignatureError) -> Self {
        Self::SignatureError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::Identifier;

    /// `SignatureError` is encoded by variant position; appending is the only safe change.
    fn discriminant_of(error: SignatureError) -> u8 {
        let bytes = bincode::encode_to_vec(error, bincode::config::standard())
            .expect("expected to encode the signature error");
        bytes[0]
    }

    #[test]
    fn signature_error_discriminants_are_frozen() {
        assert_eq!(
            discriminant_of(SignatureError::IdentityNotFoundError(
                IdentityNotFoundError::new(Identifier::from([1; 32]))
            )),
            0
        );
        assert_eq!(
            discriminant_of(SignatureError::UncompressedPublicKeyNotAllowedError(
                UncompressedPublicKeyNotAllowedError::new(65)
            )),
            12
        );
        assert_eq!(
            discriminant_of(SignatureError::ContractBoundedKeyNonBatchError(
                ContractBoundedKeyNonBatchError::new(1)
            )),
            13
        );
        assert_eq!(
            discriminant_of(SignatureError::ContractBoundedKeyOutOfBoundsError(
                ContractBoundedKeyOutOfBoundsError::new(1)
            )),
            14
        );
        assert_eq!(
            discriminant_of(SignatureError::PublicKeyBudgetExhaustedError(
                PublicKeyBudgetExhaustedError::new(1)
            )),
            15
        );
        assert_eq!(
            discriminant_of(SignatureError::PublicKeyExpiredError(
                PublicKeyExpiredError::new(1, 2, 3)
            )),
            16
        );
        assert_eq!(
            discriminant_of(
                SignatureError::PublicKeyWithLimitsCannotUpdateKeyLimitsError(
                    PublicKeyWithLimitsCannotUpdateKeyLimitsError::new(1)
                )
            ),
            17
        );
    }
}
