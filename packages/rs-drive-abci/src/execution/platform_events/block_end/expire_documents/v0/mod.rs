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
    #[inline(always)]
    pub(super) fn expire_documents_v0(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let removed = self.drive.remove_expired_documents(
            block_info,
            platform_version
                .system_limits
                .max_document_expirations_per_block,
            Some(transaction),
            platform_version,
        )?;
        if removed.deleted_documents > 0 || removed.orphaned_entries > 0 {
            tracing::debug!(
                height = block_info.height,
                deleted_documents = removed.deleted_documents,
                orphaned_entries = removed.orphaned_entries,
                "expired documents removed"
            );
        }
        Ok(())
    }
}
