use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Nothing acts on a resolved poll yet: the winner is in the poll's stored info, and the
    /// callers that open these polls take it from here.
    #[inline(always)]
    pub(super) fn on_identity_contender_vote_poll_resolved_v0(
        &self,
        block_info: &BlockInfo,
        vote_poll: &IdentityContenderVotePoll,
        winner: Option<Identifier>,
        _transaction: TransactionArg,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        tracing::debug!(
            height = block_info.height,
            %vote_poll,
            winner = ?winner,
            "identity contender vote poll resolved"
        );
        Ok(())
    }
}
