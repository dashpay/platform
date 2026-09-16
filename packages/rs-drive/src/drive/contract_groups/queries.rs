use crate::drive::contract_groups::paths::{
    contract_group_path_vec, contract_memberships_path_vec, CONTRACT_GROUP_CONTRACTS_KEY,
    CONTRACT_GROUP_DOCUMENT_TYPES_KEY, CONTRACT_GROUP_TOKENS_KEY,
    CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY, CONTRACT_MEMBERSHIPS_GROUPS_KEY,
    CONTRACT_MEMBERSHIPS_TOKENS_KEY,
};
use crate::drive::Drive;
use crate::query::{Query, QueryItem};
use grovedb::{PathQuery, SizedQuery};
use std::ops::RangeFull;

fn range_full_query() -> Query {
    let mut query = Query::new_with_direction(true);
    query.insert_item(QueryItem::RangeFull(RangeFull));
    query
}

/// A query over one level of subtrees, each holding leaves: `<key> -> <leaf key> -> element`.
fn two_level_query() -> Query {
    let mut query = range_full_query();
    query.set_subquery(range_full_query());
    query
}

impl Drive {
    /// The query for one contract group: its info item and every member.
    ///
    /// Returns the info item at the group's own path, whole-contract members under `Contracts`,
    /// document type members under `DocumentTypes/<contract id>` and token members under
    /// `Tokens/<contract id>`.
    pub fn contract_group_query(contract_group_id: [u8; 32]) -> PathQuery {
        let mut query = range_full_query();
        query.add_conditional_subquery(
            QueryItem::Key(CONTRACT_GROUP_CONTRACTS_KEY.to_vec()),
            None,
            Some(range_full_query()),
        );
        query.add_conditional_subquery(
            QueryItem::Key(CONTRACT_GROUP_DOCUMENT_TYPES_KEY.to_vec()),
            None,
            Some(two_level_query()),
        );
        query.add_conditional_subquery(
            QueryItem::Key(CONTRACT_GROUP_TOKENS_KEY.to_vec()),
            None,
            Some(two_level_query()),
        );
        PathQuery {
            path: contract_group_path_vec(&contract_group_id),
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// The query for the contract groups a contract belongs to, as a whole, through its
    /// document types, and through its tokens.
    pub fn contract_group_memberships_for_contract_query(contract_id: [u8; 32]) -> PathQuery {
        let mut query = range_full_query();
        query.add_conditional_subquery(
            QueryItem::Key(CONTRACT_MEMBERSHIPS_GROUPS_KEY.to_vec()),
            None,
            Some(range_full_query()),
        );
        query.add_conditional_subquery(
            QueryItem::Key(CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY.to_vec()),
            None,
            Some(two_level_query()),
        );
        query.add_conditional_subquery(
            QueryItem::Key(CONTRACT_MEMBERSHIPS_TOKENS_KEY.to_vec()),
            None,
            Some(two_level_query()),
        );
        PathQuery {
            path: contract_memberships_path_vec(&contract_id),
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }
}
