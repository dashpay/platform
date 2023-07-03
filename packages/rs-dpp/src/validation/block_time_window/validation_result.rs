// We can safely assume that this will not need to ever be updated
// @immutable
/// Holds the result of a time window validation.
///
/// This includes whether the validation was successful (`valid`) as well as
/// the start and end timestamps of the time window that was validated.
pub struct TimeWindowValidationResult {
    /// Indicates whether the validation was successful.
    pub valid: bool,
    /// The start timestamp of the time window that was validated.
    pub time_window_start: u64,
    /// The end timestamp of the time window that was validated.
    pub time_window_end: u64,
}
