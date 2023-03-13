use crate::prelude::TimestampMillis;

use super::validation_result::TimeWindowValidationResult;

pub const BLOCK_TIME_WINDOW_MINUTES: u64 = 5;
pub const BLOCK_TIME_WINDOW_MILLIS: TimestampMillis = BLOCK_TIME_WINDOW_MINUTES * 60 * 1000;

pub fn validate_time_in_block_time_window(
    last_block_header_time_millis: TimestampMillis,
    time_to_check_millis: TimestampMillis,
) -> TimeWindowValidationResult {
    let maybe_time_window_start =
        last_block_header_time_millis.checked_sub(BLOCK_TIME_WINDOW_MILLIS);
    let maybe_time_window_end = last_block_header_time_millis.checked_add(BLOCK_TIME_WINDOW_MILLIS);

    let time_window_start = maybe_time_window_start.unwrap_or(TimestampMillis::MIN);
    let time_window_end = maybe_time_window_end.unwrap_or(TimestampMillis::MAX);

    let valid = maybe_time_window_start.is_some()
        && maybe_time_window_end.is_some()
        && (time_to_check_millis >= time_window_start && time_to_check_millis <= time_window_end);

    TimeWindowValidationResult {
        time_window_start,
        time_window_end,
        valid,
    }
}
