use crate::error::fee::FeeError;
use crate::error::Error;

mod calculate_fee;
/// Operations
pub mod op;
/// A record of the fee calculations a thread makes
pub mod record;

/// Get overflow error
pub fn get_overflow_error(str: &'static str) -> Error {
    Error::Fee(FeeError::Overflow(str))
}
