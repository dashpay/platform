use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::drive::votes::resolved::vote_polls::ResolvedVotePollWithVotes;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    pub(super) fn clean_up_after_vote_polls_end_v0(
        &self,
        vote_polls: &BTreeMap<TimestampMillis, Vec<ResolvedVotePollWithVotes>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Create a vector to hold the references to the contested document resource vote polls
        let mut contested_polls: Vec<(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )> = Vec::new();

        // Iterate over the vote polls and match on the enum variant
        for (end_date, vote_polls_for_time) in vote_polls {
            for vote_poll in vote_polls_for_time {
                match vote_poll {
                    ResolvedVotePollWithVotes::ContestedDocumentResourceVotePollWithContractInfoAndVotes(contested_poll, vote_info) => {
                        contested_polls.push((contested_poll, end_date, vote_info));
                    } // Add more match arms here for other types of vote polls in the future
                }
            }
        }

        if !contested_polls.is_empty() {
            // Call the function to clean up contested document resource vote polls
            self.clean_up_after_contested_resources_vote_polls_end(
                contested_polls,
                transaction,
                platform_version,
            )
        } else {
            Ok(())
        }
    }
}
