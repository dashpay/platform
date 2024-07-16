use crate::error::Error;
use crate::error::fee::FeeError;

pub mod op;
mod calculate_fee;

/// Get overflow error
pub fn get_overflow_error(str: &'static str) -> Error {
    Error::Fee(FeeError::Overflow(str))
}