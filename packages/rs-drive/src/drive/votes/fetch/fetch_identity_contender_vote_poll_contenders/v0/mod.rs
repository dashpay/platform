use crate::drive::votes::paths::RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32;
use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderWithTally;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::identity_contender_vote_poll_state_query::IdentityContenderVotePollStateQuery;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_identity_contender_vote_poll_contenders_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<IdentityContenderWithTally>, Error> {
        // Contenders only: the range starts after the abstain key, so neither the stored info
        // nor the abstain tally is read
        let query = IdentityContenderVotePollStateQuery {
            vote_poll_id: vote_poll.unique_id()?,
            limit,
            start_at: Some((RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, false)),
        };
        Ok(query
            .execute_no_proof(self, transaction, &mut vec![], platform_version)?
            .contenders)
    }
}
