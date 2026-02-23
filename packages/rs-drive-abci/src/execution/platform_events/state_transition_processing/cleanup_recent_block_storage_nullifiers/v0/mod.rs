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
    /// Version 0 implementation of cleaning up expired compacted nullifier entries.
    pub(super) fn cleanup_recent_block_storage_nullifiers_v0(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.cleanup_expired_nullifiers(
            block_info.time_ms,
            Some(transaction),
            platform_version,
        )?;

        Ok(())
    }
}
