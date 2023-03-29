use crate::consensus::{basic::IndexError, fee::FeeError, signature::SignatureError};
use platform_value::Value;

use crate::errors::{
    abstract_state_error::StateError, consensus::basic::BasicError, consensus::ConsensusError,
    DataTriggerError,
};

pub trait ErrorWithCode {
    // Returns the Error Code
    fn get_code(&self) -> u32;
}

impl ErrorWithCode for ConsensusError {
    fn get_code(&self) -> u32 {
        match self {
            // Decoding
            Self::ProtocolVersionParsingError { .. } => 1000,
            Self::SerializedObjectParsingError { .. } => 1001,

            // DataContract
            Self::IncompatibleRe2PatternError { .. } => 1009,

            Self::JsonSchemaError(_) => 1005,
            Self::UnsupportedProtocolVersionError(_) => 1002,
            Self::IncompatibleProtocolVersionError(_) => 1003,

            // Identity
            Self::DuplicatedIdentityPublicKeyBasicError(_) => 1029,
            Self::DuplicatedIdentityPublicKeyBasicIdError(_) => 1030,
            Self::IdentityAssetLockProofLockedTransactionMismatchError(_) => 1031,
            Self::IdentityAssetLockTransactionIsNotFoundError(_) => 1032,
            Self::IdentityAssetLockTransactionOutPointAlreadyExistsError(_) => 1033,
            Self::IdentityAssetLockTransactionOutputNotFoundError(_) => 1034,
            Self::InvalidAssetLockProofCoreChainHeightError(_) => 1035,
            Self::InvalidAssetLockProofTransactionHeightError(_) => 1036,
            Self::InvalidAssetLockTransactionOutputReturnSize(_) => 1037,
            Self::InvalidIdentityAssetLockTransactionError(_) => 1038,
            Self::InvalidIdentityAssetLockTransactionOutputError(_) => 1039,
            Self::InvalidIdentityPublicKeyDataError(_) => 1040,
            Self::InvalidInstantAssetLockProofError(_) => 1041,
            Self::InvalidInstantAssetLockProofSignatureError(_) => 1042,
            Self::MissingMasterPublicKeyError(_) => 1046,
            Self::InvalidIdentityPublicKeySecurityLevelError(_) => 1047,
            Self::IdentityInsufficientBalanceError(_) => 4024,
            Self::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => 4025,
            Self::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => 4026,
            Self::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(_) => 4027,

            Self::StateError(e) => e.get_code(),
            Self::BasicError(e) => e.get_code(),
            Self::SignatureError(e) => e.get_code(),
            Self::FeeError(e) => e.get_code(),

            Self::IdentityAlreadyExistsError(_) => 4011,

            #[cfg(test)]
            ConsensusError::TestConsensusError(_) => 1000,
            ConsensusError::ValueError(_) => 5000,
        }
    }
}

impl ErrorWithCode for StateError {
    fn get_code(&self) -> u32 {
        match *self {
            // Document
            Self::DocumentAlreadyPresentError { .. } => 4004,
            Self::DocumentNotFoundError { .. } => 4005,
            Self::DocumentOwnerIdMismatchError { .. } => 4006,
            Self::DocumentTimestampsMismatchError { .. } => 4007,
            Self::DocumentTimestampWindowViolationError { .. } => 4008,
            Self::DuplicateUniqueIndexError { .. } => 4009,
            Self::InvalidDocumentRevisionError { .. } => 4010,
            // Data contract
            Self::DataContractAlreadyPresentError { .. } => 4000,
            Self::DataTriggerError(ref e) => e.get_code(),

            // Identity
            Self::IdentityPublicKeyDisabledAtWindowViolationError { .. } => 4012,
            Self::IdentityPublicKeyIsReadOnlyError { .. } => 4017,
            Self::InvalidIdentityPublicKeyIdError { .. } => 4018,
            Self::InvalidIdentityRevisionError { .. } => 4019,
            Self::MaxIdentityPublicKeyLimitReachedError { .. } => 4020,
            Self::DuplicatedIdentityPublicKeyError { .. } => 4021,
            Self::DuplicatedIdentityPublicKeyIdError { .. } => 4022,
            Self::IdentityPublicKeyIsDisabledError { .. } => 4023,
        }
    }
}

impl ErrorWithCode for DataTriggerError {
    fn get_code(&self) -> u32 {
        match *self {
            // Data Contract - Data Trigger
            Self::DataTriggerConditionError { .. } => 4001,
            Self::DataTriggerExecutionError { .. } => 4002,
            Self::DataTriggerInvalidResultError { .. } => 4003,
        }
    }
}

impl ErrorWithCode for BasicError {
    fn get_code(&self) -> u32 {
        match *self {
            // Document
            Self::DataContractNotPresent { .. } => 1018,
            Self::InvalidDocumentTypeError { .. } => 1024,
            Self::MissingDocumentTransitionTypeError { .. } => 1027,
            Self::MissingDocumentTransitionActionError { .. } => 1026,
            Self::MissingDocumentTypeError => 1028,
            Self::InvalidDocumentTransitionIdError { .. } => 1023,
            Self::InvalidDocumentTransitionActionError { .. } => 1022,

            Self::DuplicateDocumentTransitionsWithIdsError { .. } => 1019,
            Self::DuplicateDocumentTransitionsWithIndicesError { .. } => 1020,
            Self::MissingDataContractIdError { .. } => 1025,
            Self::InvalidIdentifierError { .. } => 1006,

            // Data contract
            Self::JsonSchemaCompilationError { .. } => 1004,
            Self::InvalidDataContractVersionError { .. } => 1050,
            Self::DataContractMaxDepthExceedError { .. } => 1007,
            Self::DuplicateIndexNameError { .. } => 1048,
            Self::InvalidJsonSchemaRefError { .. } => 1014,
            Self::InconsistentCompoundIndexDataError { .. } => 1021,
            Self::DataContractImmutablePropertiesUpdateError { .. } => 1052,
            Self::IncompatibleDataContractSchemaError { .. } => 1051,

            Self::DataContractUniqueIndicesChangedError { .. } => 1053,
            // TODO  -  they don't have error codes in  https://github.com/dashevo/platform/blob/25ab6d8a38880eaff6ac119126b2ee5991b2a5aa/packages/js-dpp/lib/errors/consensus/codes.js
            Self::DataContractHaveNewUniqueIndexError { .. } => 0,
            Self::DataContractInvalidIndexDefinitionUpdateError { .. } => 0,
            Self::IndexError(ref e) => e.get_code(),
            Self::InvalidDataContractIdError { .. } => 1011,

            // State Transition
            Self::InvalidStateTransitionTypeError { .. } => 1043,
            Self::MissingStateTransitionTypeError => 1044,
            Self::StateTransitionMaxSizeExceededError { .. } => 1045,

            // Identity
            Self::InvalidIdentityKeySignatureError { .. } => 1056,
        }
    }
}

impl ErrorWithCode for IndexError {
    fn get_code(&self) -> u32 {
        match *self {
            Self::UniqueIndicesLimitReachedError { .. } => 1017,
            Self::SystemPropertyIndexAlreadyPresentError { .. } => 1015,
            Self::UndefinedIndexPropertyError { .. } => 1016,
            Self::InvalidIndexPropertyTypeError { .. } => 1013,
            Self::InvalidIndexedPropertyConstraintError { .. } => 1012,
            Self::InvalidCompoundIndexError { .. } => 1010,
            Self::DuplicateIndexError { .. } => 1008,
        }
    }
}

impl ErrorWithCode for SignatureError {
    fn get_code(&self) -> u32 {
        match *self {
            Self::IdentityNotFoundError { .. } => 2000,
            Self::InvalidIdentityPublicKeyTypeError { .. } => 2001,
            Self::InvalidStateTransitionSignatureError => 2002,
            Self::MissingPublicKeyError { .. } => 2003,
            Self::InvalidSignaturePublicKeySecurityLevelError { .. } => 2004,
            Self::WrongPublicKeyPurposeError { .. } => 2005,
            Self::PublicKeyIsDisabledError { .. } => 2006,
            Self::PublicKeySecurityLevelNotMetError { .. } => 2007,
        }
    }
}

impl ErrorWithCode for FeeError {
    fn get_code(&self) -> u32 {
        match *self {
            Self::BalanceIsNotEnoughError { .. } => 3000,
        }
    }
}

fn create_consensus_error(code: u32, args: Value) -> ConsensusError {
    /*
        // Decoding
    1000: ProtocolVersionParsingError,
    1001: SerializedObjectParsingError,

    // General
    1002: UnsupportedProtocolVersionError,
    1003: IncompatibleProtocolVersionError,
    1004: JsonSchemaCompilationError,
    1005: JsonSchemaError,
    1006: InvalidIdentifierError,

    // Data Contract
    1007: DataContractMaxDepthExceedError,
    1008: DuplicateIndexError,
    1009: IncompatibleRe2PatternError,
    1010: InvalidCompoundIndexError,
    1011: InvalidDataContractIdError,
    1012: InvalidIndexedPropertyConstraintError,
    1013: InvalidIndexPropertyTypeError,
    1014: InvalidJsonSchemaRefError,
    1015: SystemPropertyIndexAlreadyPresentError,
    1016: UndefinedIndexPropertyError,
    1017: UniqueIndicesLimitReachedError,
    1048: DuplicateIndexNameError,
    1050: InvalidDataContractVersionError,
    1051: IncompatibleDataContractSchemaError,
    1052: DataContractImmutablePropertiesUpdateError,
    1053: DataContractUniqueIndicesChangedError,
    1054: DataContractInvalidIndexDefinitionUpdateError,
    1055: DataContractHaveNewUniqueIndexError,

    // Document
    1018: DataContractNotPresentError,
    1019: DuplicateDocumentTransitionsWithIdsError,
    1020: DuplicateDocumentTransitionsWithIndicesError,
    1021: InconsistentCompoundIndexDataError,
    1022: InvalidDocumentTransitionActionError,
    1023: InvalidDocumentTransitionIdError,
    1024: InvalidDocumentTypeError,
    1025: MissingDataContractIdError,
    1026: MissingDocumentTransitionActionError,
    1027: MissingDocumentTransitionTypeError,
    1028: MissingDocumentTypeError,

    // Identity
    1029: DuplicatedIdentityPublicKeyError,
    1030: DuplicatedIdentityPublicKeyIdError,
    1031: IdentityAssetLockProofLockedTransactionMismatchError,
    1032: IdentityAssetLockTransactionIsNotFoundError,
    1033: IdentityAssetLockTransactionOutPointAlreadyExistsError,
    1034: IdentityAssetLockTransactionOutputNotFoundError,
    1035: InvalidAssetLockProofCoreChainHeightError,
    1036: InvalidAssetLockProofTransactionHeightError,
    1037: InvalidAssetLockTransactionOutputReturnSizeError,
    1038: InvalidIdentityAssetLockTransactionError,
    1039: InvalidIdentityAssetLockTransactionOutputError,
    1040: InvalidIdentityPublicKeyDataError,
    1041: InvalidInstantAssetLockProofError,
    1042: InvalidInstantAssetLockProofSignatureError,
    1046: MissingMasterPublicKeyError,
    1047: InvalidIdentityPublicKeySecurityLevelError,
    1056: InvalidIdentityKeySignatureError,

    // State Transition
    1043: InvalidStateTransitionTypeError,
    1044: MissingStateTransitionTypeError,
    1045: StateTransitionMaxSizeExceededError,

    /**
     * Signature
     */

    2000: IdentityNotFoundError,
    2001: InvalidIdentityPublicKeyTypeError,
    2002: InvalidStateTransitionSignatureError,
    2003: MissingPublicKeyError,
    2004: InvalidSignaturePublicKeySecurityLevelError,
    2005: WrongPublicKeyPurposeError,
    2006: PublicKeyIsDisabledError,
    2007: PublicKeySecurityLevelNotMetError,

    /**
     * Fee
     */

    3000: BalanceIsNotEnoughError,

    /**
     * State
     */

    // Data Contract
    4000: DataContractAlreadyPresentError,
    4001: DataTriggerConditionError,
    4002: DataTriggerExecutionError,
    4003: DataTriggerInvalidResultError,

    // Document
    4004: DocumentAlreadyPresentError,
    4005: DocumentNotFoundError,
    4006: DocumentOwnerIdMismatchError,
    4007: DocumentTimestampsMismatchError,
    4008: DocumentTimestampWindowViolationError,
    4009: DuplicateUniqueIndexError,
    4010: InvalidDocumentRevisionError,

    // Identity
    4011: IdentityAlreadyExistsError,
    4012: IdentityPublicKeyDisabledAtWindowViolationError,
    4017: IdentityPublicKeyIsReadOnlyError,
    4018: InvalidIdentityPublicKeyIdError,
    4019: InvalidIdentityRevisionError,
    4020: StateMaxIdentityPublicKeyLimitReachedError,
    4021: DuplicatedIdentityPublicKeyStateError,
    4022: DuplicatedIdentityPublicKeyIdStateError,
    4023: IdentityPublicKeyIsDisabledError,
       */
    match code {}
}
