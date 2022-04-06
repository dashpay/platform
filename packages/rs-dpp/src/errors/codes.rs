use super::{abstract_state_error::StateError, DataTriggerError};
pub trait ErrorWithCode {
    // Returns the Error Code
    fn get_code(&self) -> Option<u32>;
}

impl ErrorWithCode for StateError {
    fn get_code(&self) -> Option<u32> {
        match *self {
            // Document
            Self::DocumentAlreadyPresentError { .. } => Some(4004),
            Self::DocumentNotFoundError { .. } => Some(4005),
            Self::DocumentOwnerMismatchError { .. } => Some(4006),
            Self::DocumentTimestampMismatchError { .. } => Some(4007),
            Self::DocumentTimestampWindowViolationError { .. } => Some(4008),
            Self::DuplicateUniqueIndexError { .. } => Some(4009),
            Self::InvalidDocumentRevisionError { .. } => Some(4010),
            // Data contract
            Self::DataContractAlreadyPresentError { .. } => Some(4000),
        }
    }
}

impl ErrorWithCode for DataTriggerError {
    fn get_code(&self) -> Option<u32> {
        match *self {
            // Data Contract - Data Trigger
            Self::DataTriggerConditionError { .. } => Some(4001),
            Self::DataTriggerExecutionError { .. } => Some(4002),
            Self::DataTriggerInvalidResultError { .. } => Some(4003),
        }
    }
}
