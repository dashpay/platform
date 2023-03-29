use jsonschema::ValidationError;
use thiserror::Error;

use crate::consensus::basic::data_contract::IncompatibleRe2PatternError;
use crate::consensus::basic::decode::ProtocolVersionParsingError;
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
use crate::errors::consensus::codes::ErrorWithCode;
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
    #[error(transparent)]
    StateError(Box<StateError>),

    #[error(transparent)]
    BasicError(Box<BasicError>),

    #[error(transparent)]
    SignatureError(SignatureError),

    #[error(transparent)]
    FeeError(FeeError),

    #[error(transparent)]
    ValueError(ValueError),

    #[cfg(test)]
    #[cfg_attr(test, error(transparent))]
    TestConsensusError(TestConsensusError),
}

impl ConsensusError {
    pub fn json_schema_error(&self) -> Option<&JsonSchemaError> {
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
            ConsensusError::IdentityInsufficientBalanceError(_) => 4024,
            ConsensusError::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => 4025,
            ConsensusError::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => 4026,
            ConsensusError::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(_) => 4027,

            ConsensusError::StateError(e) => e.get_code(),
            ConsensusError::BasicError(e) => e.code(),
            ConsensusError::SignatureError(e) => e.code(),
            ConsensusError::FeeError(e) => e.code(),

            ConsensusError::IdentityAlreadyExistsError(_) => 4011,

            // Custom error for tests
            #[cfg(test)]
            ConsensusError::TestConsensusError(_) => 1000,
            ConsensusError::ValueError(_) => 5000,
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

impl From<ValueError> for ConsensusError {
    fn from(err: ValueError) -> Self {
        Self::ValueError(err)
    }
}
