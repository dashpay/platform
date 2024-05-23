use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::document::DocumentV0Getters;
use drive::grovedb::TransactionArg;
use drive::query::vote_poll_vote_state_query::FinalizedContender;
use drive::query::VotePollsByEndDateDriveQuery;
use platform_version::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    pub(super) fn check_for_ended_contested_resource_vote_polls_v0(
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
                &mut [],
                platform_version,
            )?;

        for vote_poll in vote_polls {
            let resolved_vote_poll =
                vote_poll.resolve(&self.drive, transaction, platform_version)?;
            let document_type = resolved_vote_poll.document_type()?;
            // let's see who actually won
            let mut contenders = self.tally_votes_for_contested_document_resource_vote_poll(
                &vote_poll,
                transaction,
                platform_version,
            )?;
            let max_vote_tally = contenders.iter().map(|c| c.final_vote_tally).max();

            if let Some(max_tally) = max_vote_tally {
                // These are all the people who got top votes
                let top_contenders: Vec<FinalizedContender> = contenders
                    .into_iter()
                    .filter(|c| c.final_vote_tally == max_tally)
                    .map(|contender| {
                        FinalizedContender::try_from_contender_with_serialized_document(
                            contender,
                            document_type,
                            platform_version,
                        )
                    })
                    .collect()?;
                // Now we sort by the document creation date
                let maybe_top_contender = top_contenders.into_iter().max_by(|a, b| {
                    a.document
                        .created_at()
                        .cmp(&b.document.created_at())
                        .then_with(|| {
                            // Second criterion: length of the serialized document
                            a.document.id().cmp(&b.document.id())
                            // Alternatively, you can use another field, such as identity_id
                            // a.identity_id.cmp(&b.identity_id)
                        })
                });

                // We award the document to the top contender
            }
        }

        Ok(())
    }
}
