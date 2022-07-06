use crate::consensus::basic::IndexError;

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
            Self::InvalidIdentityPublicKeyDataError(_) => 1040,
            Self::InvalidIdentityPublicKeySecurityLevelError(_) => 1047,

            Self::StateError(e) => e.get_code(),
            Self::BasicError(e) => e.get_code(),
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
            Self::DataContractContPresent { .. } => 1018,
            Self::InvalidDocumentTypeError { .. } => 1024,
            Self::MissingDocumentTypeError { .. } => 1028,
            // Data contract
            Self::JsonSchemaCompilationError { .. } => 1004,
            Self::InvalidDataContractVersionError { .. } => 4013,
            Self::DataContractMaxDepthExceedError { .. } => 1007,
            Self::DuplicateIndexNameError { .. } => 1048,
            Self::InvalidJsonSchemaRefError { .. } => 1014,
            Self::InconsistentCompoundIndexDataError { .. } => 1021,
            Self::IndexError(ref e) => e.get_code(),
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
