use crate::error::fee::FeeError;
use crate::error::Error;

mod calculate_fee;
pub mod op;

/// Get overflow error
pub fn get_overflow_error(str: &'static str) -> Error {
    Error::Fee(FeeError::Overflow(str))
}
