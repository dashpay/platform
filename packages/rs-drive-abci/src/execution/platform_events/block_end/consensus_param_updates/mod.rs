use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::block_execution_outcome::v0::BlockExecutionOutcome;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::types::ConsensusParams;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Creates an instance of [ConsensusParams] if there are any consensus param updates are
    /// required based on [BlockExecutionOutcome]
    pub fn consensus_param_updates(
        &self,
        block_execution_outcome: &BlockExecutionOutcome,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusParams>, Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .consensus_param_updates
        {
            0 => self.consensus_param_updates_v0(block_execution_outcome, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "consensus_param_updates".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
