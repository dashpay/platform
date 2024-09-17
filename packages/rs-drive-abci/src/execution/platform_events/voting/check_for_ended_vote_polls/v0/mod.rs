use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::Network::Testnet;
use dpp::block::block_info::BlockInfo;
use dpp::document::DocumentV0Getters;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedContender;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use drive::drive::votes::paths::vote_end_date_queries_tree_path_vec;
use drive::drive::votes::resolved::vote_polls::resolve::VotePollResolver;
use drive::drive::votes::resolved::vote_polls::{ResolvedVotePoll, ResolvedVotePollWithVotes};
use drive::grovedb::{QueryItem, TransactionArg};
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use drive::query::{PathQuery, Query, QueryResultType, VotePollsByEndDateDriveQuery};
use drive::util::common::encode::encode_u64;
use itertools::Itertools;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    pub(super) fn check_for_ended_vote_polls_v0(
        &self,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // let's start by getting the vote polls that have finished
        let (
            distribute_after_time,
            clean_up_testnet_corrupted_reference_issue,
            maximum_vote_polls_to_process,
        ) = if self.config.network == Testnet
            && last_committed_platform_state.current_protocol_version_in_consensus() == 1
            && block_platform_state.current_protocol_version_in_consensus() == 2
        {
            // We need to clean up testnet
            (
                TimestampMillis::MAX >> 1,
                true,
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .event_constants
                    .maximum_vote_polls_to_process,
            )
        } else if self.config.network == Testnet
            && block_platform_state.last_committed_block_epoch().index == 1434
            && block_info.epoch.index == 1435
        {
            // We need to clean up testnet, this time we are going to do this clean up at a specific epoch change version
            (TimestampMillis::MAX >> 1, true, 100)
        } else {
            (
                block_info.time_ms,
                false,
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .event_constants
                    .maximum_vote_polls_to_process,
            )
        };
        let vote_polls_by_timestamp =
            VotePollsByEndDateDriveQuery::execute_no_proof_for_specialized_end_time_query(
                distribute_after_time,
                maximum_vote_polls_to_process,
                &self.drive,
                transaction,
                &mut vec![],
                platform_version,
            )?;

        // Check if awarding is disabled
        let vote_polls_with_info = if clean_up_testnet_corrupted_reference_issue {
            // We need to remove all empty trees before the current time
            // This is because before this fix on testnet empty trees were left in the query by end date tree
            let path = vote_end_date_queries_tree_path_vec();
            let query =
                Query::new_single_query_item(QueryItem::RangeTo(..encode_u64(block_info.time_ms)));
            let path_query = PathQuery::new_unsized(path.clone(), query);

            let keys = self
                .drive
                .grove_get_raw_path_query(
                    &path_query,
                    transaction,
                    QueryResultType::QueryKeyElementPairResultType,
                    &mut vec![],
                    &platform_version.drive,
                )?
                .0
                .to_keys();

            for key in keys {
                self.drive.grove_delete(
                    path.as_slice().into(),
                    key.as_slice(),
                    transaction,
                    &mut vec![],
                    &platform_version.drive,
                )?;
            }

            // Skip processing, just collect resolved vote polls for cleanup
            vote_polls_by_timestamp.into_iter().map(|(end_date, vote_polls)| {
                let vote_polls_with_votes = vote_polls.into_iter().map(|vote_poll| {
                    let resolved_vote_poll = vote_poll.resolve(
                        &self.drive,
                        transaction,
                        platform_version,
                    )?;
                    match resolved_vote_poll {
                        ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                            resolved_contested_document_resource_vote_poll,
                        ) => {
                            let query = ContestedDocumentVotePollDriveQuery {
                                vote_poll: (&resolved_contested_document_resource_vote_poll).into(),
                                result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
                                offset: None,
                                limit: None, // There should be no limit here
                                start_at: None,
                                allow_include_locked_and_abstaining_vote_tally: true,
                            };

                            let query_result =
                                query.execute_no_proof(&self.drive, transaction, &mut vec![], platform_version)?;

                            let contenders_with_no_votes : Vec<_> = query_result.contenders.iter().filter(|a| a.vote_tally().is_none() || a.vote_tally().unwrap() == 0 ).collect();

                            let mut other_contenders = if contenders_with_no_votes.is_empty()
                            {
                                BTreeMap::new()
                            } else {
                                // Other contenders are only those with no votes
                                contenders_with_no_votes.into_iter().map(|contender| (TowardsIdentity(contender.identity_id()), vec![])).collect()
                            };

                            if query_result.locked_vote_tally == Some(0) {
                                other_contenders.insert(ResourceVoteChoice::Lock, vec![]);
                            }

                            if query_result.abstaining_vote_tally == Some(0) {
                                other_contenders.insert(ResourceVoteChoice::Abstain, vec![]);
                            }

                            // We need to get the votes of the sorted contenders
                            let mut identifiers_voting_for_contenders =
                                self.drive.fetch_identities_voting_for_contenders(
                                    &resolved_contested_document_resource_vote_poll,
                                    query_result.contenders.into_iter().map(|contender| contender.identity_id()).collect(),
                                    true,
                                    transaction,
                                    platform_version,
                                )?;

                            identifiers_voting_for_contenders.append(&mut other_contenders);

                            // We don't need votes, so we pass an empty BTreeMap
                            Ok(ResolvedVotePollWithVotes::ContestedDocumentResourceVotePollWithContractInfoAndVotes(
                                resolved_contested_document_resource_vote_poll,
                                identifiers_voting_for_contenders,
                            ))
                        }
                    }
                }).collect::<Result<Vec<ResolvedVotePollWithVotes>, Error>>()?;
                Ok((end_date, vote_polls_with_votes))
            }).collect::<Result<BTreeMap<TimestampMillis, Vec<ResolvedVotePollWithVotes>>, Error>>()
        } else {
            vote_polls_by_timestamp.into_iter().map(|(end_date, vote_polls)| {
                let vote_polls_with_votes = vote_polls.into_iter().map(|vote_poll| {
                    let resolved_vote_poll =
                        vote_poll.resolve(&self.drive, transaction, platform_version)?;
                    match resolved_vote_poll {
                        ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                            resolved_contested_document_resource_vote_poll,
                        ) => {
                            let document_type =
                                resolved_contested_document_resource_vote_poll.document_type()?;
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
                            // These are all the people who got top votes
                            let top_contenders: Vec<FinalizedContender> = sorted_contenders
                                .into_iter()
                                .filter(|c| c.final_vote_tally == highest_vote_tally)
                                .take(100) // Limit to the first 100 before the expensive operation
                                .map(|contender| {
                                    FinalizedContender::try_from_contender_with_serialized_document(
                                        contender,
                                        document_type,
                                        platform_version,
                                    )
                                        .map_err(Error::Protocol)
                                })
                                .collect::<Result<Vec<_>, Error>>()?;
                            // Now we sort by the document creation date
                            let maybe_top_contender = top_contenders.into_iter().max_by(|a, b| {
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
                                if result.locked_vote_tally > top_contender.final_vote_tally {
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
        }?;

        // We need to clean up the vote polls
        // This means removing it and also removing all current votes
        if !vote_polls_with_info.is_empty() {
            self.clean_up_after_vote_polls_end(
                &vote_polls_with_info,
                clean_up_testnet_corrupted_reference_issue,
                transaction,
                platform_version,
            )?;
        }

        Ok(())
    }
}
