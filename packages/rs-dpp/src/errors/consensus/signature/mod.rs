mod identity_not_found_error;
mod invalid_identity_public_key_type_error;
mod invalid_signature_public_key_security_level_error;
mod missing_public_key_error;
mod public_key_is_disabled_error;
mod public_key_security_level_not_met_error;
mod wrong_public_key_purpose_error;

use thiserror::Error;

pub use crate::consensus::signature::identity_not_found_error::IdentityNotFoundError;
pub use crate::consensus::signature::invalid_identity_public_key_type_error::InvalidIdentityPublicKeyTypeError;
pub use crate::consensus::signature::invalid_signature_public_key_security_level_error::InvalidSignaturePublicKeySecurityLevelError;
pub use crate::consensus::signature::missing_public_key_error::MissingPublicKeyError;
pub use crate::consensus::signature::public_key_is_disabled_error::PublicKeyIsDisabledError;
pub use crate::consensus::signature::public_key_security_level_not_met_error::PublicKeySecurityLevelNotMetError;
pub use crate::consensus::signature::wrong_public_key_purpose_error::WrongPublicKeyPurposeError;

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
