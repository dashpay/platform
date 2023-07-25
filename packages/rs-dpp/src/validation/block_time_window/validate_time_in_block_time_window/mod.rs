use crate::identity::TimestampMillis;
use crate::validation::block_time_window::validate_time_in_block_time_window::v0::validate_time_in_block_time_window_v0;
use crate::validation::block_time_window::validation_result::TimeWindowValidationResult;
use crate::version::PlatformVersion;
use crate::NonConsensusError;

pub mod v0;

pub fn validate_time_in_block_time_window(
    last_block_header_time_millis: TimestampMillis,
    time_to_check_millis: TimestampMillis,
    average_block_spacing_ms: u64, //in the event of very long blocks we need to add this
    platform_version: &PlatformVersion,
) -> Result<TimeWindowValidationResult, NonConsensusError> {
    match platform_version
        .dpp
        .validation
        .validate_time_in_block_time_window
    {
        0 => validate_time_in_block_time_window_v0(
            last_block_header_time_millis,
            time_to_check_millis,
            average_block_spacing_ms,
        ),
        version => Err(NonConsensusError::UnknownVersionMismatch {
            method: "validate_time_in_block_time_window".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
