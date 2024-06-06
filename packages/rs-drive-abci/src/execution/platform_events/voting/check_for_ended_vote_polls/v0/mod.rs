use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use drive::drive::votes::resolved::vote_polls::resolve::VotePollResolver;
use drive::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use drive::grovedb::TransactionArg;
use drive::query::vote_poll_vote_state_query::FinalizedContender;
use drive::query::VotePollsByEndDateDriveQuery;
use itertools::Itertools;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    pub(super) fn check_for_ended_vote_polls_v0(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // let's start by getting the vote polls that have finished
        let vote_polls =
            VotePollsByEndDateDriveQuery::execute_no_proof_for_specialized_end_time_query(
                block_info.time_ms,
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .event_constants
                    .maximum_vote_polls_to_process,
                &self.drive,
                transaction,
                &mut vec![],
                platform_version,
            )?;

        for (end_date, vote_polls) in vote_polls {
            for vote_poll in &vote_polls {
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

                        let restrict_to_only_fetch_contenders = if sorted_contenders
                            .last()
                            .map(|last| last.final_vote_tally > 0)
                            .unwrap_or_default()
                        {
                            None
                        } else {
                            // We only take the first 100
                            Some(
                                sorted_contenders
                                    .iter()
                                    .take(100)
                                    .filter_map(|contender| {
                                        if contender.final_vote_tally > 0 {
                                            Some(contender.identity_id)
                                        } else {
                                            None
                                        }
                                    })
                                    .collect(),
                            )
                        };

                        // We need to get the votes of the sorted contenders
                        let identifiers_voting_for_contenders =
                            self.drive.fetch_identities_voting_for_contenders(
                                &resolved_contested_document_resource_vote_poll,
                                restrict_to_only_fetch_contenders,
                                transaction,
                                platform_version,
                            )?;

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
                                .map_err(Error::Drive)
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
                        if let Some(top_contender) = maybe_top_contender {
                            // let's check to make sure the lock votes didn't win it
                            // if the lock is tied with the top contender the top contender gets it
                            if result.locked_vote_tally > top_contender.final_vote_tally {
                                self.lock_contested_resource(
                                    block_info,
                                    &resolved_contested_document_resource_vote_poll,
                                    transaction,
                                    platform_version,
                                )?;
                            } else {
                                // We want to keep a record of how everyone voted
                                self.keep_record_of_vote_poll(
                                    block_info,
                                    &top_contender,
                                    &resolved_contested_document_resource_vote_poll,
                                    transaction,
                                    platform_version,
                                )?;
                                // We award the document to the winner of the vote poll
                                self.award_document_to_winner(
                                    block_info,
                                    top_contender,
                                    resolved_contested_document_resource_vote_poll,
                                    transaction,
                                    platform_version,
                                )?;
                            }
                        }
                    }
                }
            }

            // We need to clean up the vote poll
            // This means removing it and also removing all current votes
            self.clean_up_after_vote_polls_end(
                block_info,
                &vote_polls,
                transaction,
                platform_version,
            )?;
        }
        Ok(())
    }
}
