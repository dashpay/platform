use thiserror::Error;

use crate::{
    identity::{KeyID, KeyType, Purpose, SecurityLevel},
    prelude::Identifier,
};

#[derive(Error, Debug)]
pub enum SignatureError {
    #[error("Public key {public_key_id} doesn't exist")]
    MissingPublicKeyError { public_key_id: u64 },

    #[error("Unsupported signature type {public_key_type}. Please use ECDSA (0) or BLS (1) keys to sign the state transition")]
    InvalidIdentityPublicKeyTypeError { public_key_type: KeyType },

    #[error("Invalid State Transition signature")]
    InvalidStateTransitionSignatureError,

    #[error("Identity {identity_id} not found")]
    IdentityNotFoundError { identity_id: Identifier },

    #[error("Invalid public key security level {public_key_security_level}. The state transition requires {required_key_security_level}")]
    InvalidSignaturePublicKeySecurityLevelError {
        public_key_security_level: SecurityLevel,
        required_key_security_level: SecurityLevel,
    },

    #[error("Identity key {public_key_id} is disabled")]
    PublicKeyIsDisabledError { public_key_id: KeyID },

    #[error("Invalid security level {public_key_security_level}. This state transition requires at least {required_security_level}")]
    PublicKeySecurityLevelNotMetError {
        public_key_security_level: SecurityLevel,
        required_security_level: SecurityLevel,
    },

    #[error("Invalid identity key purpose {public_key_purpose}. This state transition requires {key_purpose_requirement}")]
    WrongPublicKeyPurposeError {
        public_key_purpose: Purpose,
        key_purpose_requirement: Purpose,
    },
}
