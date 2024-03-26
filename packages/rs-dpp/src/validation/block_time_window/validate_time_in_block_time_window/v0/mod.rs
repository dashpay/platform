use crate::validation::block_time_window::validation_result::TimeWindowValidationResult;
use crate::{prelude::TimestampMillis, NonConsensusError};

pub const BLOCK_TIME_WINDOW_MINUTES: u64 = 5;
pub const BLOCK_TIME_WINDOW_MILLIS: u64 = BLOCK_TIME_WINDOW_MINUTES * 60 * 1000;

/// Validates whether the provided timestamp (`time_to_check_millis`) falls within a calculated
/// time window based on the timestamp of the last block header (`last_block_header_time_millis`).
/// The window is calculated as `BLOCK_TIME_WINDOW_MILLIS` before and after
/// the `last_block_header_time_millis`, plus the `average_block_spacing_ms` to the end window.
///
/// Returns a `TimeWindowValidationResult` with information about the calculated time window
/// and a validity flag indicating whether the time_to_check falls within this window.
///
/// # Arguments
///
/// * `last_block_header_time_millis` - The timestamp in milliseconds of the last block header.
/// * `time_to_check_millis` - The timestamp in milliseconds that needs to be checked against the time window.
/// * `average_block_spacing_ms` - The average spacing in milliseconds between blocks, added to the end of the time window.
///
/// # Errors
///
/// If any arithmetic operation (subtraction or addition) overflows, an `NonConsensusError::Overflow` error is returned.
#[inline(always)]
pub(super) fn validate_time_in_block_time_window_v0(
    last_block_header_time_millis: TimestampMillis,
    time_to_check_millis: TimestampMillis,
    average_block_spacing_ms: u64, //in the event of very long blocks we need to add this
) -> Result<TimeWindowValidationResult, NonConsensusError> {
    let time_window_start = last_block_header_time_millis
        .checked_sub(BLOCK_TIME_WINDOW_MILLIS)
        .ok_or(NonConsensusError::Overflow(
            "calculation of start window failed",
        ))?;
    let time_window_end = last_block_header_time_millis
        .checked_add(BLOCK_TIME_WINDOW_MILLIS)
        .ok_or(NonConsensusError::Overflow(
            "calculation of end window failed: block time window overflow",
        ))?
        .checked_add(average_block_spacing_ms)
        .ok_or(NonConsensusError::Overflow(
            "calculation of end window failed: average block spacing overflow",
        ))?;

    let valid =
        time_to_check_millis >= time_window_start && time_to_check_millis <= time_window_end;

    Ok(TimeWindowValidationResult {
        time_window_start,
        time_window_end,
        valid,
    })
}
