use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod v0;
mod v1;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls and finalizes each one: awards the winner, records the
    /// finished poll and cleans it up, all on the block transaction.
    ///
    /// Generation 0 selects the winner in this crate and inserts it through the generic
    /// document insert. Generation 1 (protocol version 17) delegates selection and insert to
    /// `Drive::award_contested_document_vote_poll`, the native award operation that
    /// re-derives the winner from state and takes no contender.
    pub(in crate::execution) fn check_for_ended_vote_polls(
        &self,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .check_for_ended_vote_polls
        {
            0 => self.check_for_ended_vote_polls_v0(
                last_committed_platform_state,
                block_platform_state,
                block_info,
                transaction,
                platform_version,
            ),
            1 => self.check_for_ended_vote_polls_v1(
                block_platform_state,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "check_for_ended_vote_polls".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
