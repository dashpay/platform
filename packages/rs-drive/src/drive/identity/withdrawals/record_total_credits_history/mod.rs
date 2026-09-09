mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Records the current total credits in Platform in the total credits history under the
    /// withdrawals tree, keyed by the block time, if it differs from the latest recorded one,
    /// and on doing so prunes entries that the day-lagged daily withdrawal limit can no longer
    /// read: everything older than the entry
    /// [`Drive::fetch_total_credits_in_platform_a_day_ago`] resolves to for this block, at most
    /// `prune_limit` entries per call (`0` disables pruning). Called every block; writes only
    /// when the total changed, since the limit reads the latest entry at least a day old and an
    /// entry describes the total until the next one.
    pub fn record_total_credits_history(
        &self,
        block_info: &BlockInfo,
        prune_limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .record_total_credits_history
        {
            Some(0) => self.record_total_credits_history_v0(
                block_info,
                prune_limit,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "record_total_credits_history".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "record_total_credits_history".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
