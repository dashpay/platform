use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use thiserror::Error;

use crate::consensus::basic::data_contract::data_contract_max_depth_exceed_error::DataContractMaxDepthExceedError;
use crate::consensus::basic::data_contract::{
    DataContractHaveNewUniqueIndexError, DataContractImmutablePropertiesUpdateError,
    DataContractInvalidIndexDefinitionUpdateError, DataContractUniqueIndicesChangedError,
    DuplicateIndexError, DuplicateIndexNameError, IncompatibleDataContractSchemaError,
    IncompatibleRe2PatternError, InvalidCompoundIndexError, InvalidDataContractIdError,
    InvalidDataContractVersionError, InvalidIndexPropertyTypeError,
    InvalidIndexedPropertyConstraintError, InvalidJsonSchemaRefError,
    SystemPropertyIndexAlreadyPresentError, UndefinedIndexPropertyError,
    UniqueIndicesLimitReachedError,
};
use crate::consensus::basic::decode::{ProtocolVersionParsingError, SerializedObjectParsingError};
use crate::consensus::basic::document::{
    DataContractNotPresentError, DuplicateDocumentTransitionsWithIdsError,
    DuplicateDocumentTransitionsWithIndicesError, InconsistentCompoundIndexDataError,
    InvalidDocumentTransitionActionError, InvalidDocumentTransitionIdError,
    InvalidDocumentTypeError,
};
use crate::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyBasicError, DuplicatedIdentityPublicKeyIdBasicError,
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockTransactionIsNotFoundError,
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError, InvalidAssetLockProofCoreChainHeightError,
    InvalidAssetLockProofTransactionHeightError, InvalidAssetLockTransactionOutputReturnSizeError,
    InvalidIdentityAssetLockTransactionError, InvalidIdentityAssetLockTransactionOutputError,
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError, InvalidIdentityKeySignatureError,
    InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError,
    InvalidInstantAssetLockProofError, InvalidInstantAssetLockProofSignatureError,
    MissingMasterPublicKeyError, NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
};
use crate::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use crate::consensus::basic::state_transition::{
    InvalidStateTransitionTypeError, StateTransitionMaxSizeExceededError,
};
use crate::consensus::basic::{IncompatibleProtocolVersionError, UnsupportedProtocolVersionError};
use crate::consensus::ConsensusError;

use crate::consensus::basic::json_schema_error::JsonSchemaError;
use platform_value::Error as ValueError;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum BasicError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    // Decoding
    #[error(transparent)]
    ProtocolVersionParsingError(ProtocolVersionParsingError),

    #[error(transparent)]
    SerializedObjectParsingError(SerializedObjectParsingError),

    #[error(transparent)]
    UnsupportedProtocolVersionError(UnsupportedProtocolVersionError),

    #[error(transparent)]
    IncompatibleProtocolVersionError(IncompatibleProtocolVersionError),

    // Structure error
    #[error("{0}")]
    JsonSchemaCompilationError(String),

    #[error(transparent)]
    JsonSchemaError(JsonSchemaError),

    #[error(transparent)]
    InvalidIdentifierError(InvalidIdentifierError),

    #[error(transparent)]
    #[serde(skip)] // TODO: Figure this out
    ValueError(ValueError),

    // DataContract
    #[error(transparent)]
    DataContractMaxDepthExceedError(DataContractMaxDepthExceedError),

    #[error(transparent)]
    DuplicateIndexError(DuplicateIndexError),

    #[error(transparent)]
    IncompatibleRe2PatternError(IncompatibleRe2PatternError),

    #[error(transparent)]
    InvalidCompoundIndexError(InvalidCompoundIndexError),

    #[error(transparent)]
    InvalidDataContractIdError(InvalidDataContractIdError),

    #[error(transparent)]
    InvalidIndexedPropertyConstraintError(InvalidIndexedPropertyConstraintError),

    #[error(transparent)]
    InvalidIndexPropertyTypeError(InvalidIndexPropertyTypeError),

    #[error(transparent)]
    InvalidJsonSchemaRefError(InvalidJsonSchemaRefError),

    #[error(transparent)]
    SystemPropertyIndexAlreadyPresentError(SystemPropertyIndexAlreadyPresentError),

    #[error(transparent)]
    UndefinedIndexPropertyError(UndefinedIndexPropertyError),

    #[error(transparent)]
    UniqueIndicesLimitReachedError(UniqueIndicesLimitReachedError),

    #[error(transparent)]
    DuplicateIndexNameError(DuplicateIndexNameError),

    #[error(transparent)]
    InvalidDataContractVersionError(InvalidDataContractVersionError),

    #[error(transparent)]
    IncompatibleDataContractSchemaError(IncompatibleDataContractSchemaError),

    #[error(transparent)]
    DataContractImmutablePropertiesUpdateError(DataContractImmutablePropertiesUpdateError),

    #[error(transparent)]
    DataContractUniqueIndicesChangedError(DataContractUniqueIndicesChangedError),

    #[error(transparent)]
    DataContractInvalidIndexDefinitionUpdateError(DataContractInvalidIndexDefinitionUpdateError),

    #[error(transparent)]
    DataContractHaveNewUniqueIndexError(DataContractHaveNewUniqueIndexError),

    // Document
    #[error(transparent)]
    DataContractNotPresentError(DataContractNotPresentError),

    #[error(transparent)]
    DuplicateDocumentTransitionsWithIdsError(DuplicateDocumentTransitionsWithIdsError),

    #[error(transparent)]
    DuplicateDocumentTransitionsWithIndicesError(DuplicateDocumentTransitionsWithIndicesError),

    #[error(transparent)]
    InconsistentCompoundIndexDataError(InconsistentCompoundIndexDataError),

    #[error(transparent)]
    InvalidDocumentTransitionActionError(InvalidDocumentTransitionActionError),

    #[error(transparent)]
    InvalidDocumentTransitionIdError(InvalidDocumentTransitionIdError),

    #[error(transparent)]
    InvalidDocumentTypeError(InvalidDocumentTypeError),

    #[error("$dataContractId is not present")]
    MissingDataContractIdBasicError,

    #[error("$action is not present")]
    MissingDocumentTransitionActionError,

    #[error("$type is not present")]
    MissingDocumentTransitionTypeError,

    #[error("$type is not present")]
    MissingDocumentTypeError,

    // Identity
    #[error(transparent)]
    DuplicatedIdentityPublicKeyBasicError(DuplicatedIdentityPublicKeyBasicError),

    #[error(transparent)]
    DuplicatedIdentityPublicKeyIdBasicError(DuplicatedIdentityPublicKeyIdBasicError),

    #[error(transparent)]
    IdentityAssetLockProofLockedTransactionMismatchError(
        IdentityAssetLockProofLockedTransactionMismatchError,
    ),

    #[error(transparent)]
    IdentityAssetLockTransactionIsNotFoundError(IdentityAssetLockTransactionIsNotFoundError),

    #[error(transparent)]
    IdentityAssetLockTransactionOutPointAlreadyExistsError(
        IdentityAssetLockTransactionOutPointAlreadyExistsError,
    ),

    #[error(transparent)]
    IdentityAssetLockTransactionOutputNotFoundError(
        IdentityAssetLockTransactionOutputNotFoundError,
    ),

    #[error(transparent)]
    InvalidAssetLockProofCoreChainHeightError(InvalidAssetLockProofCoreChainHeightError),

    #[error(transparent)]
    InvalidAssetLockProofTransactionHeightError(InvalidAssetLockProofTransactionHeightError),

    #[error(transparent)]
    InvalidAssetLockTransactionOutputReturnSizeError(
        InvalidAssetLockTransactionOutputReturnSizeError,
    ),

    #[error(transparent)]
    InvalidIdentityAssetLockTransactionError(InvalidIdentityAssetLockTransactionError),

    #[error(transparent)]
    InvalidIdentityAssetLockTransactionOutputError(InvalidIdentityAssetLockTransactionOutputError),

    #[error(transparent)]
    InvalidIdentityPublicKeyDataError(InvalidIdentityPublicKeyDataError),

    #[error(transparent)]
    InvalidInstantAssetLockProofError(InvalidInstantAssetLockProofError),

    #[error(transparent)]
    InvalidInstantAssetLockProofSignatureError(InvalidInstantAssetLockProofSignatureError),

    #[error(transparent)]
    MissingMasterPublicKeyError(MissingMasterPublicKeyError),

    #[error(transparent)]
    InvalidIdentityPublicKeySecurityLevelError(InvalidIdentityPublicKeySecurityLevelError),

    #[error(transparent)]
    InvalidIdentityKeySignatureError(InvalidIdentityKeySignatureError),

    #[error(transparent)]
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError(
        InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
    ),

    #[error(transparent)]
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError(
        InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    ),

    #[error(transparent)]
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError(
        NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
    ),

    // State Transition
    #[error(transparent)]
    InvalidStateTransitionTypeError(InvalidStateTransitionTypeError),

    #[error("State transition type is not present")]
    MissingStateTransitionTypeError,

    #[error(transparent)]
    StateTransitionMaxSizeExceededError(StateTransitionMaxSizeExceededError),
}

impl From<ValueError> for ConsensusError {
    fn from(err: ValueError) -> Self {
        Self::BasicError(BasicError::ValueError(err))
    }
}

impl From<BasicError> for ConsensusError {
    fn from(error: BasicError) -> Self {
        Self::BasicError(error)
    }
}
