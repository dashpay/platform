use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollResult;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use drive::grovedb::TransactionArg;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Acts on the result of a yes/no poll that just finished, in the same block, before its
    /// votes are cleaned up. The feature that opened the poll recognises it by its resource
    /// path. Nothing opens a yes/no poll yet, so version 0 acts on nothing.
    pub(in crate::execution) fn on_yes_no_vote_poll_finished(
        &self,
        block_info: &BlockInfo,
        vote_poll: &YesNoVotePoll,
        result: &YesNoVotePollResult,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .on_yes_no_vote_poll_finished
        {
            0 => self.on_yes_no_vote_poll_finished_v0(
                block_info,
                vote_poll,
                result,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "on_yes_no_vote_poll_finished".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
