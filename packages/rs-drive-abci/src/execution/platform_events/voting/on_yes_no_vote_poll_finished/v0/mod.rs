use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollResult;
use dpp::voting::vote_polls::yes_no_vote_poll::{YesNoVotePoll, YesNoVotePollPurpose};
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Acts on a finished poll by its purpose. Every purpose is matched, so a feature that
    /// opens polls for a new purpose decides here what its result does.
    #[inline(always)]
    pub(super) fn on_yes_no_vote_poll_finished_v0(
        &self,
        _block_info: &BlockInfo,
        vote_poll: &YesNoVotePoll,
        _result: &YesNoVotePollResult,
        _transaction: TransactionArg,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match vote_poll.purpose {
            // Signaling only: the result stays in the poll's stored info, and nothing acts on
            // it.
            YesNoVotePollPurpose::DecisionProposal { .. } => Ok(()),
        }
    }
}
