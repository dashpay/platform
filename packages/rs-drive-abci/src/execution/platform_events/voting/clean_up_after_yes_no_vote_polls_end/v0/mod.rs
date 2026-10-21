use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use dpp::ProtocolError;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    pub(super) fn clean_up_after_yes_no_vote_polls_end_v0(
        &self,
        block_info: &BlockInfo,
        vote_polls: Vec<(
            &YesNoVotePoll,
            &TimestampMillis,
            &BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>,
        )>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut operations = vec![];

        // The polls' entries in the end date index, which contested polls share: an end date's
        // tree goes only once nothing of either kind is left under it.
        let end_date_entries = vote_polls
            .iter()
            .map(|(vote_poll, end_date, _)| Ok((vote_poll.unique_id()?, **end_date)))
            .collect::<Result<Vec<(Identifier, TimestampMillis)>, ProtocolError>>()?;
        self.drive.remove_vote_poll_end_date_query_operations(
            end_date_entries.as_slice(),
            &mut operations,
            transaction,
            platform_version,
        )?;

        // The votes and the three vote trees of every poll.
        let polls: Vec<&YesNoVotePoll> = vote_polls
            .iter()
            .map(|(vote_poll, _, _)| *vote_poll)
            .collect();
        self.drive.remove_yes_no_vote_poll_votes_operations(
            polls.as_slice(),
            &mut operations,
            platform_version,
        )?;

        // Each voter's entries in the identity votes index.
        let mut poll_ids_by_voter: BTreeMap<&Identifier, Vec<Identifier>> = BTreeMap::new();
        for (vote_poll, _, voters_by_choice) in &vote_polls {
            let vote_poll_id = vote_poll.unique_id()?;
            for voters in voters_by_choice.values() {
                for voter in voters {
                    poll_ids_by_voter
                        .entry(voter)
                        .or_default()
                        .push(vote_poll_id);
                }
            }
        }
        for (voter, vote_poll_ids) in poll_ids_by_voter {
            let vote_poll_ids: Vec<&Identifier> = vote_poll_ids.iter().collect();
            self.drive
                .remove_yes_no_vote_references_given_by_identity_operations(
                    voter,
                    vote_poll_ids.as_slice(),
                    &mut operations,
                    transaction,
                    platform_version,
                )?;
        }

        // What is left of the prefunded balances goes to processing, as for contested polls.
        self.empty_vote_poll_prefunded_balances_operations_v1(
            polls
                .iter()
                .map(|vote_poll| vote_poll.specialized_balance_id()),
            block_info,
            &mut operations,
            transaction,
            platform_version,
        )?;

        if !operations.is_empty() {
            self.drive.apply_batch_low_level_drive_operations(
                None,
                transaction,
                operations,
                &mut vec![],
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}
