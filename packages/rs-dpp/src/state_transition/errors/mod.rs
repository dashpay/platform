#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
pub mod invalid_identity_public_key_type_error;
#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
pub mod invalid_signature_public_key_error;
#[cfg(feature = "state-transition-validation")]
pub mod public_key_mismatch_error;
#[cfg(feature = "state-transition-validation")]
pub mod public_key_security_level_not_met_error;
#[cfg(any(
    all(feature = "state-transitions", feature = "validation"),
    feature = "state-transition-validation"
))]
pub mod state_transition_error;
#[cfg(feature = "state-transition-validation")]
pub mod state_transition_is_not_signed_error;
#[cfg(any(
    all(feature = "state-transitions", feature = "validation"),
    feature = "state-transition-validation",
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
pub mod wrong_public_key_purpose_error;

#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
pub use invalid_identity_public_key_type_error::InvalidIdentityPublicKeyTypeError;
#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
pub use invalid_signature_public_key_error::InvalidSignaturePublicKeyError;
#[cfg(feature = "state-transition-validation")]
pub use public_key_mismatch_error::PublicKeyMismatchError;
#[cfg(feature = "state-transition-validation")]
pub use public_key_security_level_not_met_error::PublicKeySecurityLevelNotMetError;
#[cfg(any(
    all(feature = "state-transitions", feature = "validation"),
    feature = "state-transition-validation"
))]
pub use state_transition_error::StateTransitionError;
#[cfg(feature = "state-transition-validation")]
pub use state_transition_is_not_signed_error::StateTransitionIsNotSignedError;
#[cfg(any(
    all(feature = "state-transitions", feature = "validation"),
    feature = "state-transition-validation",
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
pub use wrong_public_key_purpose_error::WrongPublicKeyPurposeError;
