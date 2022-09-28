use crate::consensus::{basic::IndexError, fee::FeeError, signature::SignatureError};

use super::{
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
            Self::DuplicatedIdentityPublicKeyError(_) => 1029,
            Self::DuplicatedIdentityPublicKeyIdError(_) => 1030,
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
            Self::IdentityInsufficientBalanceError(_) => 4023,

            Self::StateError(e) => e.get_code(),
            Self::BasicError(e) => e.get_code(),
            Self::SignatureError(e) => e.get_code(),
            Self::FeeError(e) => e.get_code(),

            Self::IdentityAlreadyExistsError(_) => 4011,

            #[cfg(test)]
            ConsensusError::TestConsensusError(_) => 1000,
        }
    }
}

impl ErrorWithCode for StateError {
    fn get_code(&self) -> u32 {
        match *self {
            // Document
            Self::DocumentAlreadyPresentError { .. } => 4004,
            Self::DocumentNotFoundError { .. } => 4005,
            Self::DocumentOwnerMismatchError { .. } => 4006,
            Self::DocumentTimestampMismatchError { .. } => 4007,
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
            Self::MaxIdentityPublicKeyLimitReached { .. } => 4020,
            Self::DuplicatedIdentityPublicKeyError { .. } => 4021,
            Self::DuplicatedIdentityPublicKeyIdError { .. } => 4022,
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
            Self::MissingDocumentTypeError { .. } => 1027,
            Self::MissingDocumentTransitionActionError { .. } => 1026,
            Self::InvalidDocumentTransitionIdError { .. } => 1023,
            Self::InvalidDocumentTransitionActionError { .. } => 1022,

            Self::DuplicateDocumentTransitionsWithIdsError { .. } => 1019,
            Self::MissingDataContractIdError => 1025,
            Self::InvalidIdentifierError { .. } => 1006,

            // Data contract
            Self::JsonSchemaCompilationError { .. } => 1004,
            Self::InvalidDataContractVersionError { .. } => 4013,
            Self::DataContractMaxDepthExceedError { .. } => 1007,
            Self::DuplicateIndexNameError { .. } => 1048,
            Self::InvalidJsonSchemaRefError { .. } => 1014,
            Self::InconsistentCompoundIndexDataError { .. } => 1021,
            Self::DataContractImmutablePropertiesUpdateError { .. } => 1052,
            Self::IncompatibleDataContractSchemaError { .. } => 1051,

            Self::DataContractUniqueIndicesChangedError { .. } => 4016,
            // TODO  -  they don't have error codes in  https://github.com/dashevo/platform/blob/25ab6d8a38880eaff6ac119126b2ee5991b2a5aa/packages/js-dpp/lib/errors/consensus/codes.js
            Self::DataContractHaveNewUniqueIndexError { .. } => 0,
            Self::DataContractInvalidIndexDefinitionUpdateError { .. } => 0,
            Self::IndexError(ref e) => e.get_code(),
            Self::IdentityNotFoundError { .. } => 2000,

            // State Transition
            Self::InvalidStateTransitionTypeError { .. } => 1043,
            Self::MissingStateTransitionTypeError => 1044,
            Self::StateTransitionMaxSizeExceededError { .. } => 1045,

            // Identity
            Self::InvalidIdentityPublicKeySignatureError { .. } => 1056,
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
