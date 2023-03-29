use thiserror::Error;

use crate::consensus::basic::data_contract::{
    DataContractHaveNewUniqueIndexError, DataContractImmutablePropertiesUpdateError,
    DataContractInvalidIndexDefinitionUpdateError, DataContractUniqueIndicesChangedError,
    DuplicateIndexError, DuplicateIndexNameError, IncompatibleDataContractSchemaError,
    IncompatibleRe2PatternError, InvalidCompoundIndexError, InvalidDataContractIdError,
    InvalidIndexPropertyTypeError, InvalidIndexedPropertyConstraintError,
    InvalidJsonSchemaRefError, SystemPropertyIndexAlreadyPresentError, UndefinedIndexPropertyError,
    UniqueIndicesLimitReachedError,
};
use crate::consensus::basic::decode::ProtocolVersionParsingError;
use crate::consensus::basic::document::{
    DuplicateDocumentTransitionsWithIdsError, DuplicateDocumentTransitionsWithIndicesError,
    InconsistentCompoundIndexDataError, InvalidDocumentTransitionActionError,
    InvalidDocumentTransitionIdError, InvalidDocumentTypeError,
};
use crate::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError,
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
use crate::consensus::basic::invalid_data_contract_version_error::InvalidDataContractVersionError;
use crate::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use crate::consensus::basic::state_transition::{
    InvalidStateTransitionTypeError, StateTransitionMaxSizeExceededError,
};
use crate::consensus::basic::{
    IncompatibleProtocolVersionError, JsonSchemaError, UnsupportedProtocolVersionError,
};
use crate::data_contract::errors::DataContractNotPresentError;
use crate::data_contract::state_transition::errors::MissingDataContractIdError;
use crate::prelude::*;

#[derive(Error, Debug)]
pub enum BasicError {
    #[error(transparent)]
    ProtocolVersionParsingError(ProtocolVersionParsingError),

    #[error("Parsing of serialized object failed due to: {parsing_error}")]
    SerializedObjectParsingError { parsing_error: anyhow::Error },

    #[error(transparent)]
    JsonSchemaError(JsonSchemaError),
    #[error(transparent)]
    UnsupportedProtocolVersionError(UnsupportedProtocolVersionError),
    #[error(transparent)]
    IncompatibleProtocolVersionError(IncompatibleProtocolVersionError),

    #[error(transparent)]
    DataContractNotPresent(DataContractNotPresentError),

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
    InvalidAssetLockProofTransactionHeightError(InvalidAssetLockProofTransactionHeightError),

    #[error(transparent)]
    IncompatibleRe2PatternError(IncompatibleRe2PatternError),

    #[error(transparent)]
    DuplicatedIdentityPublicKeyBasicIdError(DuplicatedIdentityPublicKeyIdError),
    #[error(transparent)]
    InvalidIdentityPublicKeyDataError(InvalidIdentityPublicKeyDataError),
    #[error(transparent)]
    InvalidIdentityPublicKeySecurityLevelError(InvalidIdentityPublicKeySecurityLevelError),
    #[error(transparent)]
    DuplicatedIdentityPublicKeyBasicError(DuplicatedIdentityPublicKeyError),

    #[error(transparent)]
    InvalidDataContractVersionError(InvalidDataContractVersionError),

    #[error("JSON Schema depth is greater than {0}")]
    DataContractMaxDepthExceedError(usize),

    // Document
    #[error(transparent)]
    InvalidDocumentTypeError(InvalidDocumentTypeError),

    #[error(transparent)]
    DuplicateIndexNameError(DuplicateIndexNameError),

    #[error(transparent)]
    InvalidJsonSchemaRefError(InvalidJsonSchemaRefError),

    #[error(transparent)]
    UniqueIndicesLimitReachedError(UniqueIndicesLimitReachedError),

    #[error(transparent)]
    SystemPropertyIndexAlreadyPresentError(SystemPropertyIndexAlreadyPresentError),

    #[error(transparent)]
    UndefinedIndexPropertyError(UndefinedIndexPropertyError),

    #[error(transparent)]
    InvalidIndexPropertyTypeError(InvalidIndexPropertyTypeError),

    #[error(transparent)]
    InvalidIndexedPropertyConstraintError(InvalidIndexedPropertyConstraintError),

    #[error(transparent)]
    InvalidCompoundIndexError(InvalidCompoundIndexError),

    #[error(transparent)]
    DuplicateIndexError(DuplicateIndexError),

    #[error("{0}")]
    JsonSchemaCompilationError(String),

    #[error(transparent)]
    InconsistentCompoundIndexDataError(InconsistentCompoundIndexDataError),

    #[error("$type is not present")]
    MissingDocumentTransitionTypeError,

    #[error("$type is not present")]
    MissingDocumentTypeError,

    #[error("$action is not present")]
    MissingDocumentTransitionActionError,

    #[error(transparent)]
    InvalidDocumentTransitionActionError(InvalidDocumentTransitionActionError),

    #[error(transparent)]
    InvalidDocumentTransitionIdError(InvalidDocumentTransitionIdError),

    #[error(transparent)]
    DuplicateDocumentTransitionsWithIdsError(DuplicateDocumentTransitionsWithIdsError),

    #[error(transparent)]
    DuplicateDocumentTransitionsWithIndicesError(DuplicateDocumentTransitionsWithIndicesError),

    #[error(transparent)]
    MissingDataContractIdError(MissingDataContractIdError),

    #[error(transparent)]
    InvalidIdentifierError(InvalidIdentifierError),

    #[error(transparent)]
    DataContractUniqueIndicesChangedError(DataContractUniqueIndicesChangedError),

    #[error(transparent)]
    DataContractInvalidIndexDefinitionUpdateError(DataContractInvalidIndexDefinitionUpdateError),

    #[error(transparent)]
    DataContractHaveNewUniqueIndexError(DataContractHaveNewUniqueIndexError),

    #[error("State transition type is not present")]
    MissingStateTransitionTypeError,

    #[error(transparent)]
    InvalidStateTransitionTypeError(InvalidStateTransitionTypeError),

    #[error(transparent)]
    StateTransitionMaxSizeExceededError(StateTransitionMaxSizeExceededError),

    #[error(transparent)]
    DataContractImmutablePropertiesUpdateError(DataContractImmutablePropertiesUpdateError),

    #[error(transparent)]
    IncompatibleDataContractSchemaError(IncompatibleDataContractSchemaError),

    #[error(transparent)]
    InvalidIdentityKeySignatureError(InvalidIdentityKeySignatureError),

    #[error(transparent)]
    InvalidDataContractIdError(InvalidDataContractIdError),
}
