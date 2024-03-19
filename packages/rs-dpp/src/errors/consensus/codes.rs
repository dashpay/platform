use crate::consensus::signature::SignatureError;
#[cfg(feature = "state-transition-validation")]
use crate::consensus::state::data_trigger::DataTriggerError;
use crate::data_contract::errors::DataContractError;

use crate::errors::consensus::{
    basic::BasicError, fee::fee_error::FeeError, state::state_error::StateError, ConsensusError,
};

pub trait ErrorWithCode {
    /// Returns the error code
    fn code(&self) -> u32;
}

impl ErrorWithCode for ConsensusError {
    fn code(&self) -> u32 {
        match self {
            Self::BasicError(e) => e.code(),
            Self::SignatureError(e) => e.code(),
            Self::StateError(e) => e.code(),
            Self::FeeError(e) => e.code(),

            #[cfg(test)]
            ConsensusError::TestConsensusError(_) => 1000,
            ConsensusError::DefaultError => 1, // this should never happen
        }
    }
}
impl ErrorWithCode for BasicError {
    fn code(&self) -> u32 {
        match self {
            // Versioning Errors: 10000-10099
            Self::UnsupportedVersionError(_) => 10000,
            Self::ProtocolVersionParsingError { .. } => 10001,
            Self::SerializedObjectParsingError { .. } => 10002,
            Self::UnsupportedProtocolVersionError(_) => 10003,
            Self::IncompatibleProtocolVersionError(_) => 10004,
            Self::VersionError(_) => 10005,

            // Structure Errors: 10100-10199
            Self::JsonSchemaCompilationError(..) => 10100,
            Self::JsonSchemaError(_) => 10101,
            Self::InvalidIdentifierError { .. } => 10102,
            Self::ValueError(_) => 10103,

            // DataContract Errors: 10200-10399
            Self::DataContractMaxDepthExceedError { .. } => 10200,
            Self::DuplicateIndexError { .. } => 10201,
            Self::IncompatibleRe2PatternError { .. } => 10202,
            Self::InvalidCompoundIndexError { .. } => 10203,
            Self::InvalidDataContractIdError { .. } => 10204,
            Self::InvalidIndexedPropertyConstraintError { .. } => 10205,
            Self::InvalidIndexPropertyTypeError { .. } => 10206,
            Self::InvalidJsonSchemaRefError { .. } => 10207,
            Self::SystemPropertyIndexAlreadyPresentError { .. } => 10208,
            Self::UndefinedIndexPropertyError { .. } => 10209,
            Self::UniqueIndicesLimitReachedError { .. } => 10210,
            Self::DuplicateIndexNameError { .. } => 10211,
            Self::InvalidDataContractVersionError { .. } => 10212,
            Self::IncompatibleDataContractSchemaError { .. } => 10213,
            Self::DataContractEmptySchemaError { .. } => 10214,
            Self::DataContractImmutablePropertiesUpdateError { .. } => 10215,
            Self::DataContractUniqueIndicesChangedError { .. } => 10216,
            Self::DataContractInvalidIndexDefinitionUpdateError { .. } => 10217,
            Self::DataContractHaveNewUniqueIndexError { .. } => 10218,
            Self::InvalidDocumentTypeRequiredSecurityLevelError { .. } => 10219,
            Self::UnknownSecurityLevelError { .. } => 10220,
            Self::UnknownStorageKeyRequirementsError { .. } => 10221,
            Self::ContractError(DataContractError::DecodingContractError { .. }) => 10222,
            Self::ContractError(DataContractError::DecodingDocumentError { .. }) => 10223,
            Self::ContractError(DataContractError::InvalidDocumentTypeError { .. }) => 10224,
            Self::ContractError(DataContractError::MissingRequiredKey(_)) => 10225,
            Self::ContractError(DataContractError::FieldRequirementUnmet(_)) => 10226,
            Self::ContractError(DataContractError::KeyWrongType(_)) => 10227,
            Self::ContractError(DataContractError::ValueWrongType(_)) => 10228,
            Self::ContractError(DataContractError::ValueDecodingError(_)) => 10229,
            Self::ContractError(DataContractError::EncodingDataStructureNotSupported(_)) => 10230,
            Self::ContractError(DataContractError::InvalidContractStructure(_)) => 10231,
            Self::ContractError(DataContractError::DocumentTypeNotFound(_)) => 10232,
            Self::ContractError(DataContractError::DocumentTypeFieldNotFound(_)) => 10233,
            Self::ContractError(DataContractError::ReferenceDefinitionNotFound(_)) => 10234,
            Self::ContractError(DataContractError::DocumentOwnerIdMissing(_)) => 10235,
            Self::ContractError(DataContractError::DocumentIdMissing(_)) => 10236,
            Self::ContractError(DataContractError::Unsupported(_)) => 10237,
            Self::ContractError(DataContractError::CorruptedSerialization(_)) => 10238,
            Self::ContractError(DataContractError::JsonSchema(_)) => 10239,
            Self::ContractError(DataContractError::InvalidURI(_)) => 10240,
            Self::ContractError(DataContractError::KeyWrongBounds(_)) => 10241,
            Self::ContractError(DataContractError::KeyValueMustExist(_)) => 10242,

            // Document Errors: 10400-10499
            Self::DataContractNotPresentError { .. } => 10400,
            Self::DuplicateDocumentTransitionsWithIdsError { .. } => 10401,
            Self::DuplicateDocumentTransitionsWithIndicesError { .. } => 10402,
            Self::InconsistentCompoundIndexDataError { .. } => 10403,
            Self::InvalidDocumentTransitionActionError { .. } => 10404,
            Self::InvalidDocumentTransitionIdError { .. } => 10405,
            Self::InvalidDocumentTypeError { .. } => 10406,
            Self::MissingDataContractIdBasicError { .. } => 10407,
            Self::MissingDocumentTransitionActionError { .. } => 10408,
            Self::MissingDocumentTransitionTypeError { .. } => 10409,
            Self::MissingDocumentTypeError { .. } => 10410,
            Self::MissingPositionsInDocumentTypePropertiesError { .. } => 10411,
            Self::MaxDocumentsTransitionsExceededError { .. } => 10412,
            Self::DocumentTransitionsAreAbsentError { .. } => 10413,
            Self::IdentityContractNonceOutOfBoundsError(_) => 10414,

            // Identity Errors: 10500-10599
            Self::DuplicatedIdentityPublicKeyBasicError(_) => 10500,
            Self::DuplicatedIdentityPublicKeyIdBasicError(_) => 10501,
            Self::IdentityAssetLockProofLockedTransactionMismatchError(_) => 10502,
            Self::IdentityAssetLockTransactionIsNotFoundError(_) => 10503,
            Self::IdentityAssetLockTransactionOutPointAlreadyExistsError(_) => 10504,
            Self::IdentityAssetLockTransactionOutputNotFoundError(_) => 10505,
            Self::InvalidAssetLockProofCoreChainHeightError(_) => 10506,
            Self::InvalidAssetLockProofTransactionHeightError(_) => 10507,
            Self::InvalidAssetLockTransactionOutputReturnSizeError(_) => 10508,
            Self::InvalidIdentityAssetLockTransactionError(_) => 10509,
            Self::InvalidIdentityAssetLockTransactionOutputError(_) => 10510,
            Self::InvalidIdentityPublicKeyDataError(_) => 10511,
            Self::InvalidInstantAssetLockProofError(_) => 10512,
            Self::InvalidInstantAssetLockProofSignatureError(_) => 10513,
            Self::InvalidIdentityAssetLockProofChainLockValidationError(_) => 10514,
            Self::DataContractBoundsNotPresentError(_) => 10515,
            Self::DisablingKeyIdAlsoBeingAddedInSameTransitionError(_) => 10516,
            Self::MissingMasterPublicKeyError(_) => 10517,
            Self::TooManyMasterPublicKeyError(_) => 10518,
            Self::InvalidIdentityPublicKeySecurityLevelError(_) => 10519,
            Self::InvalidIdentityKeySignatureError { .. } => 10520,
            Self::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => 10521,
            Self::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => 10522,
            Self::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(_) => 10523,
            Self::InvalidIdentityCreditTransferAmountError(_) => 10524,
            Self::InvalidIdentityCreditWithdrawalTransitionAmountError(_) => 10525,
            Self::InvalidIdentityUpdateTransitionEmptyError(_) => 10526,
            Self::InvalidIdentityUpdateTransitionDisableKeysError(_) => 10527,
            Self::IdentityCreditTransferToSelfError(_) => 10528,
            Self::MasterPublicKeyUpdateError(_) => 10529,

            // State Transition Errors: 10600-10699
            Self::InvalidStateTransitionTypeError { .. } => 10600,
            Self::MissingStateTransitionTypeError { .. } => 10601,
            Self::StateTransitionMaxSizeExceededError { .. } => 10602,
        }
    }
}

impl ErrorWithCode for SignatureError {
    fn code(&self) -> u32 {
        match self {
            Self::IdentityNotFoundError { .. } => 20000,
            Self::InvalidIdentityPublicKeyTypeError { .. } => 20001,
            Self::InvalidStateTransitionSignatureError { .. } => 20002,
            Self::MissingPublicKeyError { .. } => 20003,
            Self::InvalidSignaturePublicKeySecurityLevelError { .. } => 20004,
            Self::WrongPublicKeyPurposeError { .. } => 20005,
            Self::PublicKeyIsDisabledError { .. } => 20006,
            Self::PublicKeySecurityLevelNotMetError { .. } => 20007,
            Self::SignatureShouldNotBePresentError(_) => 20008,
            Self::BasicECDSAError(_) => 20009,
            Self::BasicBLSError(_) => 20010,
            Self::InvalidSignaturePublicKeyPurposeError(_) => 20011,
        }
    }
}

impl ErrorWithCode for FeeError {
    fn code(&self) -> u32 {
        match self {
            Self::BalanceIsNotEnoughError { .. } => 30000,
        }
    }
}

impl ErrorWithCode for StateError {
    fn code(&self) -> u32 {
        match self {
            // Data contract Errors: 40000-40099
            Self::DataContractAlreadyPresentError { .. } => 40000,
            Self::DataContractIsReadonlyError { .. } => 40001,
            Self::DataContractConfigUpdateError { .. } => 40002,

            // Document Errors: 40100-40199
            Self::DocumentAlreadyPresentError { .. } => 40100,
            Self::DocumentNotFoundError { .. } => 40101,
            Self::DocumentOwnerIdMismatchError { .. } => 40102,
            Self::DocumentTimestampsMismatchError { .. } => 40103,
            Self::DocumentTimestampWindowViolationError { .. } => 40104,
            Self::DuplicateUniqueIndexError { .. } => 40105,
            Self::InvalidDocumentRevisionError { .. } => 40106,
            Self::DocumentTimestampsAreEqualError(_) => 40107,

            // Identity Errors: 40200-40299
            Self::IdentityAlreadyExistsError(_) => 40200,
            Self::IdentityPublicKeyIsReadOnlyError { .. } => 40201,
            Self::InvalidIdentityPublicKeyIdError { .. } => 40202,
            Self::InvalidIdentityRevisionError { .. } => 40203,
            Self::InvalidIdentityNonceError(_) => 40204,
            Self::MaxIdentityPublicKeyLimitReachedError { .. } => 40205,
            Self::DuplicatedIdentityPublicKeyStateError { .. } => 40206,
            Self::DuplicatedIdentityPublicKeyIdStateError { .. } => 40207,
            Self::IdentityPublicKeyIsDisabledError { .. } => 40208,
            Self::MissingIdentityPublicKeyIdsError { .. } => 40209,
            Self::IdentityInsufficientBalanceError(_) => 40210,
            Self::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(_) => 40211,
            Self::InvalidAssetLockProofValueError(_) => 40212,
            Self::DocumentTypeUpdateError(_) => 40213,

            // Data trigger errors: 40500-40799
            #[cfg(feature = "state-transition-validation")]
            Self::DataTriggerError(ref e) => e.code(),
        }
    }
}

#[cfg(feature = "state-transition-validation")]
impl ErrorWithCode for DataTriggerError {
    fn code(&self) -> u32 {
        match self {
            Self::DataTriggerConditionError { .. } => 40500,
            Self::DataTriggerExecutionError { .. } => 40501,
            Self::DataTriggerInvalidResultError { .. } => 40502,
        }
    }
}
