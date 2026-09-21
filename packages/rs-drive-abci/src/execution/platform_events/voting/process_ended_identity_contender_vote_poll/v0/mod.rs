use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::{
    IdentityContenderVotePollStatus, IdentityContenderVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use dpp::voting::vote_polls::VotePoll;
use drive::drive::votes::resolved::vote_polls::identity_contender_vote_poll::{
    IdentityContenderVotePollEndOutcome, IdentityContenderWithTally,
};
use drive::error::drive::DriveError;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[inline(always)]
    pub(super) fn process_ended_identity_contender_vote_poll_v0(
        &self,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        vote_poll: &IdentityContenderVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityContenderVotePollEndOutcome, Error> {
        let mut stored_info = self
            .drive
            .fetch_identity_contender_vote_poll_stored_info(
                vote_poll,
                transaction,
                platform_version,
            )?
            .ok_or(Error::Drive(drive::error::Error::Drive(
                DriveError::CorruptedDriveState(format!(
                    "identity contender vote poll {} has an end date but no stored info",
                    vote_poll
                )),
            )))?;

        let contenders = self.drive.fetch_identity_contender_vote_poll_contenders(
            vote_poll,
            Some(
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .event_constants
                    .maximum_contenders_to_consider,
            ),
            transaction,
            platform_version,
        )?;

        let winner = match stored_info.status() {
            IdentityContenderVotePollStatus::Joining => {
                if contenders.len() > 1 {
                    // The contenders are set: the masternodes vote until the vote end time
                    stored_info.start_vote_phase()?;
                    let vote_end_time_ms = stored_info.vote_end_time_ms();
                    let mut operations = self
                        .drive
                        .insert_stored_info_for_identity_contender_vote_poll_operations(
                            vote_poll,
                            stored_info,
                            platform_version,
                        )?;
                    self.drive.add_vote_poll_end_date_query_operations(
                        None,
                        VotePoll::IdentityContenderVotePoll(vote_poll.clone()),
                        vote_end_time_ms,
                        block_info,
                        &mut None,
                        &mut None,
                        &mut operations,
                        transaction,
                        platform_version,
                    )?;
                    self.drive.apply_batch_low_level_drive_operations(
                        None,
                        transaction,
                        operations,
                        &mut vec![],
                        &platform_version.drive,
                    )?;
                    return Ok(IdentityContenderVotePollEndOutcome::VotePhaseStarted);
                }
                // A single contender wins without a vote; no contender means no winner
                contenders.first().map(|contender| contender.identity_id)
            }
            IdentityContenderVotePollStatus::Voting => Self::plurality_winner(&contenders),
            IdentityContenderVotePollStatus::Resolved => {
                return Err(Error::Drive(drive::error::Error::Drive(
                    DriveError::CorruptedDriveState(format!(
                        "identity contender vote poll {} resolved already but still has an end date",
                        vote_poll
                    )),
                )));
            }
        };

        // Who voted for what, with every choice present so that the clean-up removes every
        // tree of the poll, voted for or not
        let mut votes = self
            .drive
            .fetch_identities_voting_in_identity_contender_vote_poll(
                vote_poll,
                contenders
                    .iter()
                    .map(|contender| contender.identity_id)
                    .collect(),
                true,
                transaction,
                platform_version,
            )?;
        for contender in &contenders {
            votes
                .entry(ResourceVoteChoice::TowardsIdentity(contender.identity_id))
                .or_default();
        }
        votes.entry(ResourceVoteChoice::Abstain).or_default();

        self.keep_record_of_finished_identity_contender_vote_poll(
            block_platform_state,
            block_info,
            vote_poll,
            stored_info,
            &votes,
            winner,
            transaction,
            platform_version,
        )?;

        self.on_identity_contender_vote_poll_resolved(
            block_info,
            vote_poll,
            winner,
            transaction,
            platform_version,
        )?;

        Ok(IdentityContenderVotePollEndOutcome::Resolved { winner, votes })
    }

    /// The contender with the highest tally. A tie goes to the first contender: the earliest
    /// block time, then the lowest block height, then the lowest reference id. With no votes at
    /// all every contender ties, so the first contender wins.
    fn plurality_winner(contenders: &[IdentityContenderWithTally]) -> Option<Identifier> {
        let highest_tally = contenders
            .iter()
            .map(|contender| contender.vote_tally.unwrap_or_default())
            .max()?;
        contenders
            .iter()
            .filter(|contender| contender.vote_tally.unwrap_or_default() == highest_tally)
            .min_by_key(|contender| contender.info.tie_break_key())
            .map(|contender| contender.identity_id)
    }
}
