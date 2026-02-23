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
    /// Version 0 implementation of storing nullifiers to recent block storage.
    pub(super) fn store_nullifiers_to_recent_block_storage_v0(
        &self,
        nullifiers: &[[u8; 32]],
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.store_nullifiers_for_block(
            nullifiers,
            block_info.height,
            block_info.time_ms,
            Some(transaction),
            platform_version,
        )?;

        Ok(())
    }
}
