use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStoredInfo;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Records how an identity contender vote poll ended in its stored info: the winner, and
    /// every choice with the masternodes that voted for it and their strength.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::execution) fn keep_record_of_finished_identity_contender_vote_poll(
        &self,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        vote_poll: &IdentityContenderVotePoll,
        stored_info: IdentityContenderVotePollStoredInfo,
        votes: &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        winner: Option<Identifier>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .keep_record_of_finished_identity_contender_vote_poll
        {
            0 => self.keep_record_of_finished_identity_contender_vote_poll_v0(
                block_platform_state,
                block_info,
                vote_poll,
                stored_info,
                votes,
                winner,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "keep_record_of_finished_identity_contender_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
