use crate::drive::votes::paths::{RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, VOTING_STORAGE_TREE_KEY};
use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollPaths;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::GroveError;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    pub(super) fn fetch_identities_voting_in_identity_contender_vote_poll_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        contenders: Vec<Identifier>,
        also_fetch_abstaining_votes: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<ResourceVoteChoice, Vec<Identifier>>, Error> {
        if contenders.is_empty() && !also_fetch_abstaining_votes {
            return Ok(BTreeMap::new());
        }
        let path = vote_poll.poll_path_vec()?;
        let mut query = Query::new_with_direction(true);
        query.insert_keys(contenders.into_iter().map(|id| id.to_vec()).collect());
        if also_fetch_abstaining_votes {
            query.insert_key(RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.to_vec());
        }
        query.set_subquery_path(vec![vec![VOTING_STORAGE_TREE_KEY]]);
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
        let (results, _) = match self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &platform_version.drive,
        ) {
            Ok(results) => results,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                return Ok(BTreeMap::new());
            }
            Err(e) => return Err(e),
        };
        results
            .to_previous_of_last_path_to_keys_btree_map()
            .into_iter()
            .map(|(key, voters)| {
                let voters = voters
                    .into_iter()
                    .map(|voter| voter.try_into())
                    .collect::<Result<Vec<Identifier>, dpp::platform_value::Error>>()?;
                if key == RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32 {
                    Ok((ResourceVoteChoice::Abstain, voters))
                } else {
                    Ok((ResourceVoteChoice::TowardsIdentity(key.try_into()?), voters))
                }
            })
            .collect()
    }
}
