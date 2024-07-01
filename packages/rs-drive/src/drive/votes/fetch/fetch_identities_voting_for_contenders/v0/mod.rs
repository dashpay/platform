use crate::drive::votes::paths::{
    VotePollPaths, RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, RESOURCE_LOCK_VOTE_TREE_KEY_U8_32,
    VOTING_STORAGE_TREE_KEY,
};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    /// Fetches the identities voting for contenders.
    pub fn fetch_identities_voting_for_contenders_v0(
        &self,
        contested_document_resource_vote_poll_with_contract_info: &ContestedDocumentResourceVotePollWithContractInfo,
        fetch_contenders: Vec<Identifier>,
        also_fetch_abstaining_and_locked_votes: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<ResourceVoteChoice, Vec<Identifier>>, Error> {
        let path = contested_document_resource_vote_poll_with_contract_info
            .contenders_path(platform_version)?;

        let mut query = Query::new_with_direction(true);

        query.insert_keys(fetch_contenders.into_iter().map(|id| id.to_vec()).collect());
        if also_fetch_abstaining_and_locked_votes {
            query.insert_keys(vec![
                RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.to_vec(),
                RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.to_vec(),
            ]);
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

        self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &platform_version.drive,
        )?
        .0
        .to_previous_of_last_path_to_keys_btree_map()
        .into_iter()
        .map(|(key, value_array)| {
            let voters_array = value_array
                .into_iter()
                .map(|value| value.try_into())
                .collect::<Result<Vec<Identifier>, dpp::platform_value::Error>>()?;
            if key == RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32 {
                Ok((ResourceVoteChoice::Abstain, voters_array))
            } else if key == RESOURCE_LOCK_VOTE_TREE_KEY_U8_32 {
                Ok((ResourceVoteChoice::Lock, voters_array))
            } else {
                Ok((
                    ResourceVoteChoice::TowardsIdentity(key.try_into()?),
                    voters_array,
                ))
            }
        })
        .collect()
    }
}
