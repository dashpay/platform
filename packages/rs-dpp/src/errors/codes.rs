use super::abstract_state_error::StateError;
pub trait ErrorWithCode {
    // Returns the Error Code
    fn get_code(&self) -> Option<u32>;
}

impl ErrorWithCode for StateError {
    fn get_code(&self) -> Option<u32> {
        match *self {
            Self::DocumentAlreadyPresentError { .. } => Some(4004),
            Self::DocumentNotFoundError { .. } => Some(4005),
            Self::DocumentOwnerMismatchError { .. } => Some(4006),
            Self::DocumentTimestampMismatchError { .. } => Some(4007),
            Self::DocumentTimestampWindowViolationError { .. } => Some(4008),
            Self::DuplicateUniqueIndexError { .. } => Some(4009),
            Self::InvalidDocumentRevisionError { .. } => Some(4010),
        }
    }
}
