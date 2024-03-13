use crate::error::Error;
use crate::platform_types::block_execution_outcome::v0::BlockExecutionOutcome;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::types::{ConsensusParams, VersionParams};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    // TODO: implement update of other consensus params

    pub(super) fn consensus_param_updates_v0(
        &self,
        block_execution_outcome: &BlockExecutionOutcome,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusParams>, Error> {
        let consensus_param_updates =
            block_execution_outcome
                .next_block_protocol_version
                .map(|version| ConsensusParams {
                    version: Some(VersionParams {
                        app_version: version as u64,
                    }),
                    ..Default::default()
                });

        Ok(consensus_param_updates)
    }
}
