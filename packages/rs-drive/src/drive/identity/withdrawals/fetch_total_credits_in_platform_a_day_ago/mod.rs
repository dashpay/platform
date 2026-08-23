mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use dpp::prelude::TimestampMillis;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

/// A day in milliseconds: how far back the daily withdrawal limit looks for its reference total.
pub const DAY_IN_MS: TimestampMillis = 86_400_000;

/// One entry of the total credits history kept under the withdrawals tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedTotalCredits {
    /// Time of the block the total was recorded at.
    pub time_ms: TimestampMillis,
    /// Total credits in Platform at that block.
    pub total_credits: Credits,
}

impl Drive {
    /// Returns the total credits Platform held a day before `time_ms`: the history entry
    /// recorded at the latest block whose time is at least [`DAY_IN_MS`] before `time_ms`.
    ///
    /// Returns `None` while no such entry exists, i.e. while the history is younger than a
    /// day (right after it started being recorded); the daily withdrawal limit then falls back
    /// to its bootstrap rule rather than to a younger total, so the lag cannot be skipped by
    /// inflating the total before or at activation.
    pub fn fetch_total_credits_in_platform_a_day_ago(
        &self,
        time_ms: TimestampMillis,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<RecordedTotalCredits>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .fetch_total_credits_in_platform_a_day_ago
        {
            0 => self.fetch_total_credits_in_platform_a_day_ago_v0(
                time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_total_credits_in_platform_a_day_ago".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
