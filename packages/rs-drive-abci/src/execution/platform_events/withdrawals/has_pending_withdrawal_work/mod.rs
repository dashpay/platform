mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Whether the next block has withdrawal work waiting for it: untied withdrawal
    /// transactions in the queue that the next block dequeues and signs.
    ///
    /// Drives the `propose_next_block_immediately` hint in `ResponseFinalizeBlock`, so
    /// Tenderdash proposes the next height without waiting for transactions or the empty-block
    /// interval. Not consensus: a read only, it never touches the state or the app hash, and the
    /// hint is local to the node that returns it.
    pub fn has_pending_withdrawal_work(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .has_pending_withdrawal_work
        {
            0 => self.has_pending_withdrawal_work_v0(transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "has_pending_withdrawal_work".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
