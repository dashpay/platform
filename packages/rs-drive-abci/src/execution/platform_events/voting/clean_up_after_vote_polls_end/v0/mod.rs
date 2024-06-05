use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    pub(super) fn clean_up_after_vote_polls_end_v0(
        &self,
        block_info: &BlockInfo,
        vote_polls: &[VotePoll],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Create a vector to hold the references to the contested document resource vote polls
        let mut contested_polls: Vec<&ContestedDocumentResourceVotePoll> = Vec::new();

        // Iterate over the vote polls and match on the enum variant
        for vote_poll in vote_polls {
            match vote_poll {
                VotePoll::ContestedDocumentResourceVotePoll(contested_poll) => {
                    contested_polls.push(contested_poll);
                } // Add more match arms here for other types of vote polls in the future
            }
        }

        // Call the function to clean up contested document resource vote polls
        self.clean_up_after_contested_resources_vote_polls_end(
            block_info,
            &contested_polls,
            transaction,
            platform_version,
        )
    }
}
