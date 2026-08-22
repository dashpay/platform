mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Records the total credits in Platform for this block in the history the day-lagged
    /// daily withdrawal limit reads — only when it differs from the latest recorded total,
    /// since the limit reads the entry in force a day ago and an entry describes the total
    /// until the next one — pruning entries the limit can no longer use.
    ///
    /// Runs every block before withdrawals are pooled, so the first block the history exists
    /// in already has an entry to derive the limit from; blocks that leave the total untouched
    /// cost one read and no write.
    pub(in crate::execution) fn record_total_credits_history_for_withdrawals(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .record_total_credits_history_for_withdrawals
        {
            None => Ok(()),
            Some(0) => self.record_total_credits_history_for_withdrawals_v0(
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "record_total_credits_history_for_withdrawals".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
