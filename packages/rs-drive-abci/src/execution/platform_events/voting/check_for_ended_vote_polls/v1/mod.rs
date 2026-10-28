use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::platform_events::voting::clean_up_after_yes_no_vote_polls_end::FinishedYesNoVotePoll;
use crate::execution::platform_events::voting::masternode_vote_strength;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::ContestedIndexResolution;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedContender;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dpp::voting::vote_polls::yes_no_vote_poll::{
    VotingPower, YesNoMinimumVotingPower, YesNoVotePoll,
};
use dpp::voting::vote_polls::VotePoll;
use drive::drive::votes::paths::vote_end_date_queries_tree_path_vec;
use drive::drive::votes::resolved::vote_polls::resolve::VotePollResolver;
use drive::drive::votes::resolved::vote_polls::{ResolvedVotePoll, ResolvedVotePollWithVotes};
use drive::error::drive::DriveError;
use drive::error::Error as DriveCrateError;
use drive::grovedb::TransactionArg;
use drive::query::VotePollsByEndDateDriveQuery;
use drive::util::common::encode::encode_u64;
use itertools::Itertools;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls, awarding a tie to the earliest contender, after removing
    /// the end dates left with no vote poll among the first due. From protocol version 15 it
    /// also closes yes/no polls: it records their result, hands it to the feature that opened
    /// them and cleans their votes up.
    #[inline(always)]
    pub(super) fn check_for_ended_vote_polls_v1(
        &self,
        _last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // let's start by getting the vote polls that have finished. Version 0 carried two
        // testnet clean-ups of protocol versions 1 and 2; this version only runs from
        // protocol version 14, so it reads only what is due by the block time
        let distribute_after_time = block_info.time_ms;
        let maximum_vote_polls_to_process = platform_version
            .drive_abci
            .validation_and_processing
            .event_constants
            .maximum_vote_polls_to_process;
        // An end date with no vote poll under it ends nothing yet takes a slot of the read
        // below, so enough of them due before every vote poll would stall them all. Those among
        // the first due go before the read; the read then reaches the vote polls behind them.
        for empty_end_time in VotePollsByEndDateDriveQuery::execute_no_proof_empty_end_times(
            distribute_after_time,
            maximum_vote_polls_to_process,
            &self.drive,
            transaction,
            &mut vec![],
            platform_version,
        )? {
            self.drive.grove_delete(
                vote_end_date_queries_tree_path_vec().as_slice().into(),
                encode_u64(empty_end_time).as_slice(),
                transaction,
                &mut vec![],
                &platform_version.drive,
            )?;
        }
        let vote_polls_by_timestamp =
            VotePollsByEndDateDriveQuery::execute_no_proof_for_specialized_end_time_query(
                distribute_after_time,
                maximum_vote_polls_to_process,
                &self.drive,
                transaction,
                &mut vec![],
                platform_version,
            )?;

        // Yes/no polls need no resolution: they close here, before the contested polls, which
        // keep the loop below as it was. Reached from protocol version 15 only: at 14, which
        // also selects this generation, no yes/no poll is in the end date index, since opening
        // one (`Drive::open_yes_no_vote_poll`) is not active before 15, and the contested polls
        // are the polls the index returned, in the same order. Every due yes/no entry goes from
        // the index; the polls this block closes also keep their voters for the clean-up, and a
        // poll an earlier entry already closed has nothing more to clean.
        let mut yes_no_end_date_entries: Vec<(Identifier, TimestampMillis)> = vec![];
        let mut finished_yes_no_vote_polls: Vec<FinishedYesNoVotePoll> = vec![];
        let mut contested_vote_polls_by_timestamp: BTreeMap<TimestampMillis, Vec<VotePoll>> =
            BTreeMap::new();
        for (end_date, vote_polls) in vote_polls_by_timestamp {
            for vote_poll in vote_polls {
                match vote_poll {
                    VotePoll::YesNoVotePoll(yes_no_vote_poll) => {
                        let poll_id = yes_no_vote_poll.unique_id(platform_version)?;
                        yes_no_end_date_entries.push((poll_id, end_date));
                        if let Some(voters) = self.finish_yes_no_vote_poll(
                            block_platform_state,
                            block_info,
                            &yes_no_vote_poll,
                            transaction,
                            platform_version,
                        )? {
                            finished_yes_no_vote_polls.push(FinishedYesNoVotePoll {
                                poll_id,
                                prefunded_balance_id: yes_no_vote_poll
                                    .specialized_balance_id(platform_version)?,
                                voters,
                            });
                        }
                    }
                    contested_vote_poll => contested_vote_polls_by_timestamp
                        .entry(end_date)
                        .or_default()
                        .push(contested_vote_poll),
                }
            }
        }

        // Check if awarding is disabled
        let vote_polls_with_info = contested_vote_polls_by_timestamp.into_iter().map(|(end_date, vote_polls)| {
                let vote_polls_with_votes = vote_polls.into_iter().map(|vote_poll| {
                    let resolved_vote_poll =
                        vote_poll.resolve(&self.drive, transaction, platform_version)?;
                    match resolved_vote_poll {
                        ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                            resolved_contested_document_resource_vote_poll,
                        ) => {
                            let document_type =
                                resolved_contested_document_resource_vote_poll.document_type()?;
                            // A contest on an index resolved without locking always has a
                            // winner, whatever its lock tally holds
                            let locking_allowed = resolved_contested_document_resource_vote_poll
                                .index()?
                                .contested_index
                                .as_ref()
                                .map(|contested| contested.resolution)
                                != Some(ContestedIndexResolution::MasternodeVoteNoLocking);
                            // let's see who actually won
                            let result = self.tally_votes_for_contested_document_resource_vote_poll(
                                (&resolved_contested_document_resource_vote_poll).into(),
                                transaction,
                                platform_version,
                            )?;
                            let contenders = result.contenders;
                            // For each contender if there vote_tally is 1 or more we need to get their votes
                            // We don't do this for contenders with 0 votes, as there is no point.

                            let sorted_contenders: Vec<_> = contenders
                                .into_iter()
                                .sorted_by(|a, b| Ord::cmp(&b.final_vote_tally, &a.final_vote_tally))
                                .collect();

                            let (contenders_with_votes, contenders_with_no_votes): (Vec<_>, Vec<_>) = sorted_contenders.iter().partition(|a| a.final_vote_tally > 0);

                            let fetch_contenders = contenders_with_votes
                                .iter()
                                .map(|contender| contender.identity_id)
                                .collect::<Vec<_>>();
                            let mut other_contenders = if contenders_with_no_votes.is_empty()
                            {
                                BTreeMap::new()
                            } else {
                                // Other contenders are only those with no votes
                                contenders_with_no_votes.into_iter().map(|contender| (TowardsIdentity(contender.identity_id), vec![])).collect()
                            };

                            // We need to get the votes of the sorted contenders
                            let mut identifiers_voting_for_contenders =
                                self.drive.fetch_identities_voting_for_contenders(
                                    &resolved_contested_document_resource_vote_poll,
                                    fetch_contenders,
                                    true,
                                    transaction,
                                    platform_version,
                                )?;

                            identifiers_voting_for_contenders.append(&mut other_contenders);

                            let highest_vote_tally = sorted_contenders
                                .first()
                                .map(|max_voted_contender| max_voted_contender.final_vote_tally)
                                .unwrap_or_default();
                            // These are all the people who got top votes, every one of them
                            // considered (up to `maximum_contenders_to_consider`); version 0
                            // compared at most 100
                            let top_contenders: Vec<FinalizedContender> = sorted_contenders
                                .into_iter()
                                .filter(|c| c.final_vote_tally == highest_vote_tally)
                                .map(|contender| {
                                    FinalizedContender::try_from_contender_with_serialized_document(
                                        contender,
                                        document_type,
                                        platform_version,
                                    )
                                        .map_err(Error::Protocol)
                                })
                                .collect::<Result<Vec<_>, Error>>()?;
                            // A tie among the top contenders goes to the earliest document:
                            // creation time, then block height, then core height, then id.
                            // Version 0 awarded the latest.
                            let maybe_top_contender = top_contenders.into_iter().min_by(|a, b| {
                                a.document
                                    .created_at()
                                    .cmp(&b.document.created_at())
                                    .then_with(|| {
                                        a.document
                                            .created_at_block_height()
                                            .cmp(&b.document.created_at_block_height())
                                    })
                                    .then_with(|| {
                                        a.document
                                            .created_at_core_block_height()
                                            .cmp(&b.document.created_at_core_block_height())
                                    })
                                    .then_with(|| a.document.id().cmp(&b.document.id()))
                            });
                            // We award the document to the top contender
                            let winner_info = if let Some(top_contender) = maybe_top_contender {
                                // let's check to make sure the lock votes didn't win it
                                // if the lock is tied with the top contender the top contender gets it
                                if locking_allowed
                                    && result.locked_vote_tally > top_contender.final_vote_tally
                                {
                                    // the record will show it's locked
                                    ContestedDocumentVotePollWinnerInfo::Locked
                                } else {
                                    let contender_id = top_contender.identity_id;
                                    // We award the document to the winner of the vote poll
                                    self.award_document_to_winner(
                                        block_info,
                                        top_contender,
                                        &resolved_contested_document_resource_vote_poll,
                                        transaction,
                                        platform_version,
                                    )?;
                                    ContestedDocumentVotePollWinnerInfo::WonByIdentity(contender_id)
                                }
                            } else {
                                ContestedDocumentVotePollWinnerInfo::NoWinner
                            };
                            // We want to keep a record of how everyone voted
                            self.keep_record_of_finished_contested_resource_vote_poll(
                                block_platform_state,
                                block_info,
                                &resolved_contested_document_resource_vote_poll,
                                &identifiers_voting_for_contenders,
                                winner_info,
                                transaction,
                                platform_version,
                            )?;
                            Ok(ResolvedVotePollWithVotes::ContestedDocumentResourceVotePollWithContractInfoAndVotes(resolved_contested_document_resource_vote_poll, identifiers_voting_for_contenders))
                        }
                    }
                }).collect::<Result<Vec<ResolvedVotePollWithVotes>, Error>>()?;
                Ok((end_date, vote_polls_with_votes))
            }).collect::<Result<BTreeMap<TimestampMillis, Vec<ResolvedVotePollWithVotes>>, Error>>()
?;

        // We need to clean up the vote polls
        // This means removing it and also removing all current votes
        if !vote_polls_with_info.is_empty() {
            self.clean_up_after_vote_polls_end(
                block_info,
                &vote_polls_with_info,
                false,
                transaction,
                platform_version,
            )?;
        }

        // Then the yes/no polls, in a batch of their own: its read of a shared end date sees
        // what the contested clean-up left there.
        if !yes_no_end_date_entries.is_empty() {
            self.clean_up_after_yes_no_vote_polls_end(
                block_info,
                &yes_no_end_date_entries,
                &finished_yes_no_vote_polls,
                transaction,
                platform_version,
            )?;
        }

        Ok(())
    }

    /// Closes one yes/no poll whose time has come: tallies it, records the result in its
    /// stored info, hands the result to the feature that opened the poll, and returns the
    /// voters so the clean-up can remove their votes.
    ///
    /// Returns `None` for a poll that is already finished: one listed under two end dates
    /// (two openings that could not see each other) closes at the first, and its later entry
    /// is only removed. A due poll with no stored info is corrupted state: an opening writes it
    /// with the end date entry.
    fn finish_yes_no_vote_poll(
        &self,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        vote_poll: &YesNoVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<Identifier>>, Error> {
        let mut tally =
            self.tally_votes_for_yes_no_vote_poll(vote_poll, transaction, platform_version)?;
        let stored_info = match tally.stored_info.take() {
            Some(stored_info) if stored_info.result().is_some() => return Ok(None),
            Some(stored_info) => stored_info,
            None => {
                return Err(Error::Drive(DriveCrateError::Drive(
                    DriveError::CorruptedDriveState(
                        "an ended yes/no vote poll must have a record in the state".to_string(),
                    ),
                )))
            }
        };
        // A minimum given as a share of the total is measured against the masternode list of
        // the closing block, each masternode weighed as its vote is. A fixed minimum reads no
        // total.
        let total_voting_power = match vote_poll.minimum_voting_power {
            YesNoMinimumVotingPower::Absolute { .. } => 0,
            YesNoMinimumVotingPower::FractionOfTotal { .. } => block_platform_state
                .full_masternode_list()
                .values()
                .try_fold(0 as VotingPower, |total, masternode| {
                    total.checked_add(
                        masternode_vote_strength(&masternode.node_type, platform_version).into(),
                    )
                })
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "the masternode list's total voting power overflows",
                )))?,
        };
        let result = self.keep_record_of_finished_yes_no_vote_poll(
            block_info,
            vote_poll,
            stored_info,
            &tally,
            total_voting_power,
            transaction,
            platform_version,
        )?;
        self.on_yes_no_vote_poll_finished(
            block_info,
            vote_poll,
            &result,
            transaction,
            platform_version,
        )?;
        let voters = self.drive.fetch_identities_voting_in_yes_no_vote_poll(
            vote_poll,
            transaction,
            platform_version,
        )?;
        Ok(Some(voters))
    }
}
