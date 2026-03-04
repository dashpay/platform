mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Records the current shielded pool anchor if the commitment tree changed this block.
    pub(in crate::execution) fn record_shielded_pool_anchor_if_changed(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .record_shielded_pool_anchor
        {
            None => Ok(()),
            Some(0) => self.record_shielded_pool_anchor_if_changed_v0(
                block_height,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "record_shielded_pool_anchor_if_changed".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
