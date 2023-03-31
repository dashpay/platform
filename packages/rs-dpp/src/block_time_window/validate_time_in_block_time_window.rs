use crate::{prelude::TimestampMillis, NonConsensusError};

use super::validation_result::TimeWindowValidationResult;

pub const BLOCK_TIME_WINDOW_MINUTES: u64 = 5;
pub const BLOCK_TIME_WINDOW_MILLIS: u64 = BLOCK_TIME_WINDOW_MINUTES * 60 * 1000;

pub fn validate_time_in_block_time_window(
    last_block_header_time_millis: TimestampMillis,
    time_to_check_millis: TimestampMillis,
) -> Result<TimeWindowValidationResult, NonConsensusError> {
    let time_window_start = last_block_header_time_millis
        .checked_sub(BLOCK_TIME_WINDOW_MILLIS)
        .ok_or(NonConsensusError::Overflow(
            "calculation of start window failed",
        ))?;
    let time_window_end = last_block_header_time_millis
        .checked_add(BLOCK_TIME_WINDOW_MILLIS)
        .ok_or(NonConsensusError::Overflow(
            "calculation of end window failed",
        ))?;

    let valid =
        time_to_check_millis >= time_window_start && time_to_check_millis <= time_window_end;

    Ok(TimeWindowValidationResult {
        time_window_start,
        time_window_end,
        valid,
    })
}
