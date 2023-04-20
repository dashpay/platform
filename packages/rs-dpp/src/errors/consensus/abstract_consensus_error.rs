use jsonschema::ValidationError;
use thiserror::Error;

use crate::codes::ErrorWithCode;
use crate::consensus::basic::data_contract::IncompatibleRe2PatternError;
use crate::consensus::basic::decode::ProtocolVersionParsingError;
use crate::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError,
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockTransactionIsNotFoundError,
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError, InvalidAssetLockProofCoreChainHeightError,
    InvalidAssetLockProofTransactionHeightError, InvalidAssetLockTransactionOutputReturnSizeError,
    InvalidIdentityAssetLockProofChainLockValidationError,
    InvalidIdentityAssetLockTransactionError, InvalidIdentityAssetLockTransactionOutputError,
    InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError,
    InvalidInstantAssetLockProofError, InvalidInstantAssetLockProofSignatureError,
    MissingMasterPublicKeyError,
};
use crate::consensus::state::identity::IdentityAlreadyExistsError;
use crate::errors::consensus::basic;

use crate::errors::StateError;
use platform_value::Error as ValueError;

use super::basic::identity::{
    IdentityInsufficientBalanceError, InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
};
use super::fee::FeeError;
use super::signature::SignatureError;

#[derive(Error, Debug)]
//#[cfg_attr(test, derive(Clone))]
pub enum ConsensusError {
    #[error("default error")]
    DefaultError,
    #[error(transparent)]
    JsonSchemaError(basic::JsonSchemaError),
    #[error(transparent)]
    UnsupportedProtocolVersionError(basic::UnsupportedProtocolVersionError),
    #[error(transparent)]
    IncompatibleProtocolVersionError(basic::IncompatibleProtocolVersionError),
    #[error(transparent)]
    DuplicatedIdentityPublicKeyBasicIdError(DuplicatedIdentityPublicKeyIdError),
    #[error(transparent)]
    InvalidIdentityPublicKeyDataError(InvalidIdentityPublicKeyDataError),
    #[error(transparent)]
    InvalidIdentityPublicKeySecurityLevelError(InvalidIdentityPublicKeySecurityLevelError),
    #[error(transparent)]
    DuplicatedIdentityPublicKeyBasicError(DuplicatedIdentityPublicKeyError),
    #[error(transparent)]
    MissingMasterPublicKeyError(MissingMasterPublicKeyError),
    #[error(transparent)]
    IdentityAssetLockTransactionOutPointAlreadyExistsError(
        IdentityAssetLockTransactionOutPointAlreadyExistsError,
    ),
    #[error(transparent)]
    InvalidIdentityAssetLockTransactionOutputError(InvalidIdentityAssetLockTransactionOutputError),
    #[error(transparent)]
    InvalidAssetLockTransactionOutputReturnSize(InvalidAssetLockTransactionOutputReturnSizeError),
    #[error(transparent)]
    IdentityAssetLockTransactionOutputNotFoundError(
        IdentityAssetLockTransactionOutputNotFoundError,
    ),
    #[error(transparent)]
    InvalidIdentityAssetLockTransactionError(InvalidIdentityAssetLockTransactionError),
    #[error(transparent)]
    InvalidInstantAssetLockProofError(InvalidInstantAssetLockProofError),
    #[error(transparent)]
    InvalidInstantAssetLockProofSignatureError(InvalidInstantAssetLockProofSignatureError),
    #[error(transparent)]
    IdentityAssetLockProofLockedTransactionMismatchError(
        IdentityAssetLockProofLockedTransactionMismatchError,
    ),
    #[error(transparent)]
    IdentityAssetLockTransactionIsNotFoundError(IdentityAssetLockTransactionIsNotFoundError),
    #[error(transparent)]
    InvalidAssetLockProofCoreChainHeightError(InvalidAssetLockProofCoreChainHeightError),
    #[error(transparent)]
    InvalidIdentityAssetLockProofChainLockValidationError(
        InvalidIdentityAssetLockProofChainLockValidationError,
    ),
    #[error(transparent)]
    InvalidAssetLockProofTransactionHeightError(InvalidAssetLockProofTransactionHeightError),

    #[error(transparent)]
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError(
        InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    ),

    #[error(transparent)]
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError(
        InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
    ),

    #[error("{0}")]
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError(
        NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
    ),

    #[error(transparent)]
    StateError(Box<StateError>),

    #[error(transparent)]
    BasicError(Box<basic::BasicError>),

    #[error("Parsing of serialized object failed due to: {parsing_error}")]
    SerializedObjectParsingError { parsing_error: anyhow::Error },

    #[error(transparent)]
    ProtocolVersionParsingError(ProtocolVersionParsingError),

    #[error(transparent)]
    IncompatibleRe2PatternError(IncompatibleRe2PatternError),

    #[error(transparent)]
    IdentityInsufficientBalanceError(IdentityInsufficientBalanceError),

    #[error(transparent)]
    IdentityAlreadyExistsError(IdentityAlreadyExistsError),

    #[error(transparent)]
    SignatureError(SignatureError),

    #[error(transparent)]
    FeeError(FeeError),

    #[error(transparent)]
    ValueError(ValueError),

    #[cfg(test)]
    #[cfg_attr(test, error(transparent))]
    TestConsensusError(basic::TestConsensusError),
}

impl ConsensusError {
    pub fn json_schema_error(&self) -> Option<&basic::JsonSchemaError> {
        match self {
            ConsensusError::JsonSchemaError(err) => Some(err),
            _ => None,
        }
    }

    pub fn value_error(&self) -> Option<&ValueError> {
        match self {
            ConsensusError::ValueError(err) => Some(err),
            _ => None,
        }
    }

    pub fn code(&self) -> u32 {
        match self {
            // Decoding
            ConsensusError::ProtocolVersionParsingError { .. } => 1000,
            ConsensusError::SerializedObjectParsingError { .. } => 1001,
            // Data Contract
            ConsensusError::IncompatibleRe2PatternError { .. } => 1009,

            ConsensusError::JsonSchemaError(_) => 1005,
            ConsensusError::UnsupportedProtocolVersionError(_) => 1002,
            ConsensusError::IncompatibleProtocolVersionError(_) => 1003,

            // Identity
            ConsensusError::DuplicatedIdentityPublicKeyBasicError(_) => 1029,
            ConsensusError::DuplicatedIdentityPublicKeyBasicIdError(_) => 1030,
            ConsensusError::IdentityAssetLockProofLockedTransactionMismatchError(_) => 1031,
            ConsensusError::IdentityAssetLockTransactionIsNotFoundError(_) => 1032,
            ConsensusError::IdentityAssetLockTransactionOutPointAlreadyExistsError(_) => 1033,
            ConsensusError::IdentityAssetLockTransactionOutputNotFoundError(_) => 1034,
            ConsensusError::InvalidAssetLockProofCoreChainHeightError(_) => 1035,
            ConsensusError::InvalidAssetLockProofTransactionHeightError(_) => 1036,
            ConsensusError::InvalidAssetLockTransactionOutputReturnSize(_) => 1037,
            ConsensusError::InvalidIdentityAssetLockTransactionError(_) => 1038,
            ConsensusError::InvalidIdentityAssetLockTransactionOutputError(_) => 1039,
            ConsensusError::InvalidIdentityPublicKeyDataError(_) => 1040,
            ConsensusError::InvalidInstantAssetLockProofError(_) => 1041,
            ConsensusError::InvalidInstantAssetLockProofSignatureError(_) => 1042,
            ConsensusError::InvalidIdentityAssetLockProofChainLockValidationError(_) => 1043,
            ConsensusError::MissingMasterPublicKeyError(_) => 1046,
            ConsensusError::InvalidIdentityPublicKeySecurityLevelError(_) => 1047,
            ConsensusError::IdentityInsufficientBalanceError(_) => 4024,
            ConsensusError::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => 4025,
            ConsensusError::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => 4026,
            ConsensusError::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(_) => 4027,

            ConsensusError::StateError(e) => e.get_code(),
            ConsensusError::BasicError(e) => e.get_code(),
            ConsensusError::SignatureError(e) => e.get_code(),
            ConsensusError::FeeError(e) => e.get_code(),

            ConsensusError::IdentityAlreadyExistsError(_) => 4011,

            // Custom error for tests
            #[cfg(test)]
            ConsensusError::TestConsensusError(_) => 1000,
            ConsensusError::ValueError(_) => 5000,
            ConsensusError::DefaultError => 1, // we should never get the default error anyways
        }
    }
}

impl Default for ConsensusError {
    fn default() -> Self {
        ConsensusError::DefaultError
    }
}

impl<'a> From<ValidationError<'a>> for ConsensusError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self::JsonSchemaError(basic::JsonSchemaError::from(validation_error))
    }
}

impl From<crate::errors::consensus::basic::JsonSchemaError> for ConsensusError {
    fn from(json_schema_error: basic::JsonSchemaError) -> Self {
        Self::JsonSchemaError(json_schema_error)
    }
}

impl From<crate::errors::consensus::basic::UnsupportedProtocolVersionError> for ConsensusError {
    fn from(error: crate::errors::consensus::basic::UnsupportedProtocolVersionError) -> Self {
        Self::UnsupportedProtocolVersionError(error)
    }
}

impl From<basic::IncompatibleProtocolVersionError> for ConsensusError {
    fn from(error: basic::IncompatibleProtocolVersionError) -> Self {
        Self::IncompatibleProtocolVersionError(error)
    }
}

impl From<DuplicatedIdentityPublicKeyIdError> for ConsensusError {
    fn from(error: DuplicatedIdentityPublicKeyIdError) -> Self {
        Self::DuplicatedIdentityPublicKeyBasicIdError(error)
    }
}

impl From<InvalidIdentityPublicKeyDataError> for ConsensusError {
    fn from(error: InvalidIdentityPublicKeyDataError) -> Self {
        Self::InvalidIdentityPublicKeyDataError(error)
    }
}

impl From<InvalidIdentityPublicKeySecurityLevelError> for ConsensusError {
    fn from(error: InvalidIdentityPublicKeySecurityLevelError) -> Self {
        Self::InvalidIdentityPublicKeySecurityLevelError(error)
    }
}

impl From<DuplicatedIdentityPublicKeyError> for ConsensusError {
    fn from(error: DuplicatedIdentityPublicKeyError) -> Self {
        Self::DuplicatedIdentityPublicKeyBasicError(error)
    }
}

impl From<MissingMasterPublicKeyError> for ConsensusError {
    fn from(error: MissingMasterPublicKeyError) -> Self {
        Self::MissingMasterPublicKeyError(error)
    }
}

#[cfg(test)]
impl From<basic::TestConsensusError> for ConsensusError {
    fn from(error: basic::TestConsensusError) -> Self {
        Self::TestConsensusError(error)
    }
}

impl From<StateError> for ConsensusError {
    fn from(se: StateError) -> Self {
        ConsensusError::StateError(Box::new(se))
    }
}

impl From<basic::BasicError> for ConsensusError {
    fn from(se: basic::BasicError) -> Self {
        ConsensusError::BasicError(Box::new(se))
    }
}

impl From<IdentityAssetLockTransactionOutPointAlreadyExistsError> for ConsensusError {
    fn from(err: IdentityAssetLockTransactionOutPointAlreadyExistsError) -> Self {
        Self::IdentityAssetLockTransactionOutPointAlreadyExistsError(err)
    }
}

impl From<InvalidIdentityAssetLockTransactionOutputError> for ConsensusError {
    fn from(err: InvalidIdentityAssetLockTransactionOutputError) -> Self {
        Self::InvalidIdentityAssetLockTransactionOutputError(err)
    }
}

impl From<InvalidAssetLockTransactionOutputReturnSizeError> for ConsensusError {
    fn from(err: InvalidAssetLockTransactionOutputReturnSizeError) -> Self {
        Self::InvalidAssetLockTransactionOutputReturnSize(err)
    }
}

impl From<IdentityAssetLockTransactionOutputNotFoundError> for ConsensusError {
    fn from(err: IdentityAssetLockTransactionOutputNotFoundError) -> Self {
        Self::IdentityAssetLockTransactionOutputNotFoundError(err)
    }
}

impl From<InvalidIdentityAssetLockTransactionError> for ConsensusError {
    fn from(err: InvalidIdentityAssetLockTransactionError) -> Self {
        Self::InvalidIdentityAssetLockTransactionError(err)
    }
}

impl From<InvalidInstantAssetLockProofError> for ConsensusError {
    fn from(err: InvalidInstantAssetLockProofError) -> Self {
        Self::InvalidInstantAssetLockProofError(err)
    }
}

impl From<InvalidInstantAssetLockProofSignatureError> for ConsensusError {
    fn from(err: InvalidInstantAssetLockProofSignatureError) -> Self {
        Self::InvalidInstantAssetLockProofSignatureError(err)
    }
}

impl From<IdentityAssetLockProofLockedTransactionMismatchError> for ConsensusError {
    fn from(err: IdentityAssetLockProofLockedTransactionMismatchError) -> Self {
        Self::IdentityAssetLockProofLockedTransactionMismatchError(err)
    }
}

impl From<IdentityAlreadyExistsError> for ConsensusError {
    fn from(err: IdentityAlreadyExistsError) -> Self {
        Self::IdentityAlreadyExistsError(err)
    }
}

impl From<IdentityInsufficientBalanceError> for ConsensusError {
    fn from(err: IdentityInsufficientBalanceError) -> Self {
        Self::IdentityInsufficientBalanceError(err)
    }
}

impl From<SignatureError> for ConsensusError {
    fn from(err: SignatureError) -> Self {
        Self::SignatureError(err)
    }
}

impl From<FeeError> for ConsensusError {
    fn from(err: FeeError) -> Self {
        Self::FeeError(err)
    }
}

impl From<ValueError> for ConsensusError {
    fn from(err: ValueError) -> Self {
        Self::ValueError(err)
    }
}
