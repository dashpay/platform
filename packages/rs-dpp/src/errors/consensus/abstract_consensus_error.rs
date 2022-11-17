use jsonschema::ValidationError;
use thiserror::Error;

use crate::codes::ErrorWithCode;
use crate::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError,
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockTransactionIsNotFoundError,
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError, InvalidAssetLockProofCoreChainHeightError,
    InvalidAssetLockProofTransactionHeightError, InvalidAssetLockTransactionOutputReturnSizeError,
    InvalidIdentityAssetLockTransactionError, InvalidIdentityAssetLockTransactionOutputError,
    InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError,
    InvalidInstantAssetLockProofError, InvalidInstantAssetLockProofSignatureError,
    MissingMasterPublicKeyError,
};
use crate::consensus::state::identity::IdentityAlreadyExistsError;
#[cfg(test)]
use crate::errors::consensus::basic::TestConsensusError;
use crate::errors::consensus::basic::{
    BasicError, IncompatibleProtocolVersionError, JsonSchemaError, UnsupportedProtocolVersionError,
};
use crate::errors::StateError;

use super::basic::identity::{
    IdentityInsufficientBalanceError, InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
};
use super::fee::FeeError;
use super::signature::SignatureError;

#[derive(Error, Debug)]
//#[cfg_attr(test, derive(Clone))]
pub enum ConsensusError {
    #[error("{0}")]
    JsonSchemaError(JsonSchemaError),
    #[error("{0}")]
    UnsupportedProtocolVersionError(UnsupportedProtocolVersionError),
    #[error("{0}")]
    IncompatibleProtocolVersionError(IncompatibleProtocolVersionError),
    #[error("{0}")]
    DuplicatedIdentityPublicKeyIdError(DuplicatedIdentityPublicKeyIdError),
    #[error("{0}")]
    InvalidIdentityPublicKeyDataError(InvalidIdentityPublicKeyDataError),
    #[error("{0}")]
    InvalidIdentityPublicKeySecurityLevelError(InvalidIdentityPublicKeySecurityLevelError),
    #[error("{0}")]
    DuplicatedIdentityPublicKeyError(DuplicatedIdentityPublicKeyError),
    #[error("{0}")]
    MissingMasterPublicKeyError(MissingMasterPublicKeyError),
    #[error("{0}")]
    IdentityAssetLockTransactionOutPointAlreadyExistsError(
        IdentityAssetLockTransactionOutPointAlreadyExistsError,
    ),
    #[error("{0}")]
    InvalidIdentityAssetLockTransactionOutputError(InvalidIdentityAssetLockTransactionOutputError),
    #[error("{0}")]
    InvalidAssetLockTransactionOutputReturnSize(InvalidAssetLockTransactionOutputReturnSizeError),
    #[error("{0}")]
    IdentityAssetLockTransactionOutputNotFoundError(
        IdentityAssetLockTransactionOutputNotFoundError,
    ),
    #[error("{0}")]
    InvalidIdentityAssetLockTransactionError(InvalidIdentityAssetLockTransactionError),
    #[error("{0}")]
    InvalidInstantAssetLockProofError(InvalidInstantAssetLockProofError),
    #[error("{0}")]
    InvalidInstantAssetLockProofSignatureError(InvalidInstantAssetLockProofSignatureError),
    #[error("{0}")]
    IdentityAssetLockProofLockedTransactionMismatchError(
        IdentityAssetLockProofLockedTransactionMismatchError,
    ),
    #[error(transparent)]
    IdentityAssetLockTransactionIsNotFoundError(IdentityAssetLockTransactionIsNotFoundError),
    #[error(transparent)]
    InvalidAssetLockProofCoreChainHeightError(InvalidAssetLockProofCoreChainHeightError),
    #[error(transparent)]
    InvalidAssetLockProofTransactionHeightError(InvalidAssetLockProofTransactionHeightError),

    #[error("{0}")]
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError(
        InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    ),

    #[error("{0}")]
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError(
        InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
    ),

    #[error(transparent)]
    StateError(Box<StateError>),

    #[error(transparent)]
    BasicError(Box<BasicError>),

    #[error("Parsing of serialized object failed due to: {parsing_error}")]
    SerializedObjectParsingError { parsing_error: anyhow::Error },

    #[error("Can't read protocol version from serialized object: {parsing_error}")]
    ProtocolVersionParsingError { parsing_error: anyhow::Error },

    #[error("Pattern '{pattern}' at '{path}' is not not compatible with Re2:  {message}")]
    IncompatibleRe2PatternError {
        pattern: String,
        path: String,
        message: String,
    },

    #[error("{0}")]
    IdentityInsufficientBalanceError(IdentityInsufficientBalanceError),

    #[error(transparent)]
    IdentityAlreadyExistsError(IdentityAlreadyExistsError),

    #[error(transparent)]
    SignatureError(SignatureError),

    #[error(transparent)]
    FeeError(FeeError),

    #[cfg(test)]
    #[cfg_attr(test, error("{0}"))]
    TestConsensusError(TestConsensusError),
}

impl ConsensusError {
    pub fn json_schema_error(&self) -> Option<&JsonSchemaError> {
        match self {
            ConsensusError::JsonSchemaError(err) => Some(err),
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
            ConsensusError::DuplicatedIdentityPublicKeyError(_) => 1029,
            ConsensusError::DuplicatedIdentityPublicKeyIdError(_) => 1030,
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
            ConsensusError::MissingMasterPublicKeyError(_) => 1046,
            ConsensusError::InvalidIdentityPublicKeySecurityLevelError(_) => 1047,
            ConsensusError::IdentityInsufficientBalanceError(_) => 4024,
            ConsensusError::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => 4025,
            ConsensusError::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => 4026,

            ConsensusError::StateError(e) => e.get_code(),
            ConsensusError::BasicError(e) => e.get_code(),
            ConsensusError::SignatureError(e) => e.get_code(),
            ConsensusError::FeeError(e) => e.get_code(),

            ConsensusError::IdentityAlreadyExistsError(_) => 4011,

            // Custom error for tests
            #[cfg(test)]
            ConsensusError::TestConsensusError(_) => 1000,
        }
    }
}

impl<'a> From<ValidationError<'a>> for ConsensusError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self::JsonSchemaError(JsonSchemaError::from(validation_error))
    }
}

impl From<JsonSchemaError> for ConsensusError {
    fn from(json_schema_error: JsonSchemaError) -> Self {
        Self::JsonSchemaError(json_schema_error)
    }
}

impl From<UnsupportedProtocolVersionError> for ConsensusError {
    fn from(error: UnsupportedProtocolVersionError) -> Self {
        Self::UnsupportedProtocolVersionError(error)
    }
}

impl From<IncompatibleProtocolVersionError> for ConsensusError {
    fn from(error: IncompatibleProtocolVersionError) -> Self {
        Self::IncompatibleProtocolVersionError(error)
    }
}

impl From<DuplicatedIdentityPublicKeyIdError> for ConsensusError {
    fn from(error: DuplicatedIdentityPublicKeyIdError) -> Self {
        Self::DuplicatedIdentityPublicKeyIdError(error)
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
        Self::DuplicatedIdentityPublicKeyError(error)
    }
}

impl From<MissingMasterPublicKeyError> for ConsensusError {
    fn from(error: MissingMasterPublicKeyError) -> Self {
        Self::MissingMasterPublicKeyError(error)
    }
}

#[cfg(test)]
impl From<TestConsensusError> for ConsensusError {
    fn from(error: TestConsensusError) -> Self {
        Self::TestConsensusError(error)
    }
}

impl From<StateError> for ConsensusError {
    fn from(se: StateError) -> Self {
        ConsensusError::StateError(Box::new(se))
    }
}

impl From<BasicError> for ConsensusError {
    fn from(se: BasicError) -> Self {
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
