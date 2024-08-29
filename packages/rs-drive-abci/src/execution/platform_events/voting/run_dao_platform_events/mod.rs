use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod v0;
impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Runs the dao platform events
    pub(in crate::execution) fn run_dao_platform_events(
        &self,
        block_info: &BlockInfo,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .run_dao_platform_events
        {
            0 => self.run_dao_platform_events_v0(
                block_info,
                last_committed_platform_state,
                block_platform_state,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "run_dao_platform_events".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
