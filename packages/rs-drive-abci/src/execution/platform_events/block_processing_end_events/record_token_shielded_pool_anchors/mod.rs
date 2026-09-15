mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::collections::BTreeSet;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Records the anchor of every token shielded pool the block touched, if it changed, and
    /// prunes that pool's anchors older than the retention window.
    pub(in crate::execution) fn record_token_shielded_pool_anchors(
        &self,
        token_ids: &BTreeSet<[u8; 32]>,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .record_token_shielded_pool_anchors
        {
            None => Ok(()),
            Some(0) => self.record_token_shielded_pool_anchors_v0(
                token_ids,
                block_height,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "record_token_shielded_pool_anchors".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
