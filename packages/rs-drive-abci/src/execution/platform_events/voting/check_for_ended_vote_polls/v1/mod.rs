use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
use drive::drive::votes::resolved::vote_polls::resolve::VotePollResolver;
use drive::drive::votes::resolved::vote_polls::{ResolvedVotePoll, ResolvedVotePollWithVotes};
use drive::grovedb::TransactionArg;
use drive::query::VotePollsByEndDateDriveQuery;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Finalizes every contested resource vote poll whose end date the block time has reached.
    ///
    /// Generation 1 awards through `Drive::award_contested_document_vote_poll`, the native
    /// award operation: the winner is selected and its document inserted inside Drive, from
    /// state alone, so this event supplies no contender and cannot redirect an award. It then
    /// records the finalized poll with the votes it received and cleans the poll up, all on
    /// the block transaction, exactly as generation 0 did after selecting the winner itself.
    ///
    /// Generation 0's two testnet repair branches (the protocol 1 to 2 upgrade and the epoch
    /// 1434 to 1435 boundary) cannot trigger at any protocol version that selects this
    /// generation and are not carried.
    #[inline(always)]
    pub(super) fn check_for_ended_vote_polls_v1(
        &self,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let vote_polls_by_timestamp =
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

        let vote_polls_with_info = vote_polls_by_timestamp
            .into_iter()
            .map(|(end_date, vote_polls)| {
                let vote_polls_with_votes = vote_polls
                    .into_iter()
                    .map(|vote_poll| {
                        let resolved_vote_poll =
                            vote_poll.resolve(&self.drive, transaction, platform_version)?;
                        match resolved_vote_poll {
                            ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                                resolved_contested_document_resource_vote_poll,
                            ) => {
                                // The native award: Drive checks that the poll is a started
                                // contest queued at this end date, selects the winner and
                                // inserts its document, or locks, or finds no winner.
                                let outcome = self.drive.award_contested_document_vote_poll(
                                    &resolved_contested_document_resource_vote_poll,
                                    end_date,
                                    block_info,
                                    transaction,
                                    platform_version,
                                )?;

                                // For each contender with one vote or more we record who
                                // voted for them; contenders without votes are recorded
                                // with no voters.
                                let (contenders_with_votes, contenders_with_no_votes): (
                                    Vec<_>,
                                    Vec<_>,
                                ) = outcome
                                    .contenders
                                    .iter()
                                    .partition(|contender| contender.final_vote_tally > 0);

                                let fetch_contenders = contenders_with_votes
                                    .iter()
                                    .map(|contender| contender.identity_id)
                                    .collect::<Vec<_>>();

                                let mut other_contenders = contenders_with_no_votes
                                    .into_iter()
                                    .map(|contender| {
                                        (TowardsIdentity(contender.identity_id), vec![])
                                    })
                                    .collect::<BTreeMap<_, _>>();

                                let mut identifiers_voting_for_contenders =
                                    self.drive.fetch_identities_voting_for_contenders(
                                        &resolved_contested_document_resource_vote_poll,
                                        fetch_contenders,
                                        true,
                                        transaction,
                                        platform_version,
                                    )?;

                                identifiers_voting_for_contenders.append(&mut other_contenders);

                                // We want to keep a record of how everyone voted
                                self.keep_record_of_finished_contested_resource_vote_poll(
                                    block_platform_state,
                                    block_info,
                                    &resolved_contested_document_resource_vote_poll,
                                    &identifiers_voting_for_contenders,
                                    outcome.winner,
                                    transaction,
                                    platform_version,
                                )?;

                                Ok(ResolvedVotePollWithVotes::ContestedDocumentResourceVotePollWithContractInfoAndVotes(
                                    resolved_contested_document_resource_vote_poll,
                                    identifiers_voting_for_contenders,
                                ))
                            }
                        }
                    })
                    .collect::<Result<Vec<ResolvedVotePollWithVotes>, Error>>()?;
                Ok((end_date, vote_polls_with_votes))
            })
            .collect::<Result<BTreeMap<TimestampMillis, Vec<ResolvedVotePollWithVotes>>, Error>>()?;

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

        Ok(())
    }
}
