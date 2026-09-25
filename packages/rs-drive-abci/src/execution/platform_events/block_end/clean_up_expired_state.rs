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
    /// Runs the cleanups of state that expired by the block's time, after its state
    /// transitions: the compacted address balance entries of recent block storage, then the
    /// documents whose time to live has passed (protocol version 14). Each is versioned on its
    /// own.
    ///
    /// One call from `run_block_proposal` for both, rather than one each: every fallible call
    /// there reserves room for its error in that function's frame, which the deepest paths of
    /// a block run on top of, and some strategy tests run within a few kilobytes of the
    /// default 2 MiB test thread stack in debug builds.
    ///
    /// # Parameters
    /// - `block_info`: the block being processed.
    /// - `transaction`: the block's transaction.
    /// - `platform_version`: selects the version of each cleanup.
    ///
    /// # Returns
    /// `Ok(())` once both cleanups ran.
    #[inline(never)]
    pub(in crate::execution) fn clean_up_expired_state(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.cleanup_recent_block_storage_address_balances(
            block_info,
            transaction,
            platform_version,
        )?;
        self.expire_documents(block_info, transaction, platform_version)
    }
}
