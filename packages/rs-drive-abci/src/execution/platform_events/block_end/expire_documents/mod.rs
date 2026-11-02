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
    /// Deletes documents whose type declares a `ttl` once it has passed, after the block's
    /// state transitions: at most `max_document_expirations_per_block` of them, oldest first,
    /// the rest in later blocks. A transition of this block still saw every document it
    /// deletes. Nobody pays: each document prepaid its deletion when it was created.
    ///
    /// # Parameters
    /// - `block_info`: the block being processed; its time decides what has expired.
    /// - `transaction`: the block's transaction.
    /// - `platform_version`: selects the method version; `None` before protocol version 14.
    ///
    /// # Returns
    /// `Ok(())` once the expired documents this block deletes are gone.
    pub(in crate::execution) fn expire_documents(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .expire_documents
        {
            None => Ok(()),
            Some(0) => self.expire_documents_v0(block_info, transaction, platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "expire_documents".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
