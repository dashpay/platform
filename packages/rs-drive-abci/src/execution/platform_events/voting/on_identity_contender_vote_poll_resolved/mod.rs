use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use drive::grovedb::TransactionArg;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Acts on the outcome of an identity contender vote poll, once its record is kept and
    /// before its votes are cleaned up. The polls that elect moderation teams seat the winner
    /// here (issue #4865); this version only records that the poll resolved.
    pub(in crate::execution) fn on_identity_contender_vote_poll_resolved(
        &self,
        block_info: &BlockInfo,
        vote_poll: &IdentityContenderVotePoll,
        winner: Option<Identifier>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .on_identity_contender_vote_poll_resolved
        {
            0 => self.on_identity_contender_vote_poll_resolved_v0(
                block_info,
                vote_poll,
                winner,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "on_identity_contender_vote_poll_resolved".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
