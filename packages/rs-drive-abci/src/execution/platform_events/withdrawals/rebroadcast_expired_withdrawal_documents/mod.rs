use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

mod v0;
mod v1;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Rebroadcasts expired withdrawal documents if any exist.
    ///
    /// This function attempts to rebroadcast expired withdrawal documents by checking if there are
    /// any documents with the status `EXPIRED`. It updates the status of such documents to
    /// `BROADCASTED`, increments their revision, and reschedules them for broadcasting.
    ///
    /// # Parameters
    /// - `block_info`: Information about the current block (e.g., timestamp).
    /// - `transaction`: The transaction within which the rebroadcast should be executed.
    /// - `platform_version`: The version of the platform, used to determine the correct method implementation.
    ///
    /// # Returns
    /// - `Ok(())` if the rebroadcast process succeeds without issues.
    /// - `Err(ExecutionError::UnknownVersionMismatch)` if the platform version is unsupported.
    pub fn rebroadcast_expired_withdrawal_documents(
        &self,
        block_info: &BlockInfo,
        last_committed_platform_state: &PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .rebroadcast_expired_withdrawal_documents
        {
            0 => self.rebroadcast_expired_withdrawal_documents_v0(
                block_info,
                last_committed_platform_state,
                transaction,
                platform_version,
            ),
            1 => self.rebroadcast_expired_withdrawal_documents_v1(
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "rebroadcast_expired_withdrawal_documents".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
