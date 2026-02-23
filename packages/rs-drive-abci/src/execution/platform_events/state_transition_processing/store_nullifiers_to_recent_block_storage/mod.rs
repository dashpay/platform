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
    /// Stores nullifiers inserted during this block to recent block storage for sync purposes.
    pub(in crate::execution) fn store_nullifiers_to_recent_block_storage(
        &self,
        nullifiers: &[[u8; 32]],
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .store_nullifiers_to_recent_block_storage
        {
            None => Ok(()),
            Some(0) => self.store_nullifiers_to_recent_block_storage_v0(
                nullifiers,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "store_nullifiers_to_recent_block_storage".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
