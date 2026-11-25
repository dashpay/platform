use crate::drive::votes::paths::YesNoVotePollPaths;
use crate::drive::votes::YesNoAbstainVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use std::ops::RangeFull;

impl Drive {
    pub(super) fn fetch_identities_voting_in_yes_no_vote_poll_v0(
        &self,
        vote_poll: &YesNoVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Identifier>, Error> {
        let path = vote_poll.poll_path_vec(platform_version)?;
        let mut query = Query::new_with_direction(true);
        query.insert_keys(
            YesNoAbstainVoteChoice::ALL
                .iter()
                .map(|vote_choice| vec![vote_choice.to_tree_key()])
                .collect(),
        );
        query.set_subquery(Query::new_single_query_item(QueryItem::RangeFull(
            RangeFull,
        )));
        let path_query = PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };
        // The subquery's keys are the voters' pro tx hashes, under the three choice trees.
        let voters = self
            .grove_get_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )?
            .0
            .to_keys()
            .into_iter()
            .map(|voter| voter.try_into())
            .collect::<Result<Vec<Identifier>, _>>()?;
        Ok(voters)
    }
}
