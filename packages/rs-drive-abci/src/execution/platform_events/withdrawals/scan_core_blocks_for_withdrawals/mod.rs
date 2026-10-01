mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Reads the Core blocks the chain locked height has passed since the last one read: for
    /// each, records Core's credit pool balance after it (the Core-anchored withdrawal limit
    /// reads these) and dates the asset locks Platform consumed before Core mined them, which
    /// that block holds. Reads at most `core_blocks_scanned_per_block_limit` Core blocks per
    /// block, oldest first, and never one older than the band the limit reads
    /// (`core_credit_pool_window_max_blocks` back); the rest follow in the next blocks.
    ///
    /// Only chain locked Core blocks are read, so every node reads the same. Pooling calls it
    /// every block before it reads the withdrawal limits, so they see the newest Core blocks;
    /// a long jump of the chain locked height is read over several blocks.
    ///
    /// # Parameters
    ///
    /// * `block_info`: The block being executed; its Core chain locked height is the newest
    ///   Core block read.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the Core blocks are recorded, or at once when the protocol version has
    ///   no Core-anchored limit (the method version is `None`).
    /// * `Err(Error)` when the method version (or a Drive method it calls) is unknown or not
    ///   active, Core cannot be asked, or a write fails.
    pub(in crate::execution) fn scan_core_blocks_for_withdrawals(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .scan_core_blocks_for_withdrawals
        {
            None => Ok(()),
            Some(0) => {
                self.scan_core_blocks_for_withdrawals_v0(block_info, transaction, platform_version)
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "scan_core_blocks_for_withdrawals".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
