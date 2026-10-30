use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollResult;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// No feature opens a yes/no poll yet, so no feature acts on one. The moderation team
    /// challenge and charter amendment features add their arms here, matched on the poll's
    /// resource path.
    #[inline(always)]
    pub(super) fn on_yes_no_vote_poll_finished_v0(
        &self,
        _block_info: &BlockInfo,
        _vote_poll: &YesNoVotePoll,
        _result: &YesNoVotePollResult,
        _transaction: TransactionArg,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        Ok(())
    }
}
