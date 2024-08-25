use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod v0;
impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Removes the votes for removed masternodes
    pub(in crate::execution) fn remove_votes_for_removed_masternodes(
        &self,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .remove_votes_for_removed_masternodes
        {
            0 => self.remove_votes_for_removed_masternodes_v0(
                last_committed_platform_state,
                block_platform_state,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "remove_votes_for_removed_masternodes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
