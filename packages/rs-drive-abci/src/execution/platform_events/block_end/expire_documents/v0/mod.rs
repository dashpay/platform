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
    #[inline(always)]
    pub(super) fn expire_documents_v0(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let limit = platform_version
            .system_limits
            .max_document_expirations_per_block
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "expire_documents v0 runs only where the version caps expirations per block",
            )))?;
        let removed = self.drive.remove_expired_documents(
            block_info,
            limit,
            Some(transaction),
            platform_version,
        )?;
        if removed.deleted_documents > 0
            || removed.orphaned_entries > 0
            || removed.dropped_expiry_times > 0
        {
            tracing::debug!(
                height = block_info.height,
                deleted_documents = removed.deleted_documents,
                orphaned_entries = removed.orphaned_entries,
                dropped_expiry_times = removed.dropped_expiry_times,
                "expired documents removed"
            );
        }
        Ok(())
    }
}
