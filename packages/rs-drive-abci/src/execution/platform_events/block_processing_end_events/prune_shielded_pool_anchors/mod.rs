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
    /// Prunes shielded pool anchors older than the configured retention depth.
    pub(in crate::execution) fn prune_shielded_pool_anchors(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .prune_shielded_pool_anchors
        {
            None => Ok(()),
            Some(0) => {
                self.prune_shielded_pool_anchors_v0(block_height, transaction, platform_version)
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "prune_shielded_pool_anchors".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
