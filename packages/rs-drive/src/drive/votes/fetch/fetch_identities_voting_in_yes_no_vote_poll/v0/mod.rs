use crate::drive::votes::paths::YesNoVotePollPaths;
use crate::drive::votes::YesNoAbstainVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    pub(super) fn fetch_identities_voting_in_yes_no_vote_poll_v0(
        &self,
        vote_poll: &YesNoVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>, Error> {
        let path = vote_poll.poll_path_vec()?;
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
        let voters_by_choice_key = self
            .grove_get_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryPathKeyElementTrioResultType,
                &mut vec![],
                &platform_version.drive,
            )?
            .0
            .to_last_path_to_key_elements_btree_map();
        let mut voters_by_choice: BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>> =
            YesNoAbstainVoteChoice::ALL
                .iter()
                .map(|vote_choice| (*vote_choice, vec![]))
                .collect();
        for (choice_key, voters) in voters_by_choice_key {
            let vote_choice = choice_key
                .first()
                .filter(|_| choice_key.len() == 1)
                .and_then(|key| YesNoAbstainVoteChoice::from_tree_key(*key))
                .ok_or_else(|| {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "unexpected key {} in yes/no vote poll tree",
                        hex::encode(&choice_key)
                    )))
                })?;
            let voters = voters
                .into_keys()
                .map(|voter| voter.try_into())
                .collect::<Result<Vec<Identifier>, dpp::platform_value::Error>>()?;
            voters_by_choice.insert(vote_choice, voters);
        }
        Ok(voters_by_choice)
    }
}
