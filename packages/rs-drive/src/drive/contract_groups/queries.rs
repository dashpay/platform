use crate::drive::contract_groups::paths::{
    contract_group_contracts_path_vec, contract_group_document_types_path_vec,
    contract_group_path_vec, contract_group_tokens_path_vec, contract_memberships_path_vec,
    CONTRACT_GROUP_INFO_KEY, CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY,
    CONTRACT_MEMBERSHIPS_GROUPS_KEY, CONTRACT_MEMBERSHIPS_TOKENS_KEY,
};
use crate::drive::contract_groups::types::ContractGroupMembersQuery;
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

/// A query over one level of subtrees continuing after `(first level key, second level key)`:
/// every leaf of every later subtree, and only the leaves after the second level key inside the
/// subtree of the first level key itself.
fn two_level_query_after(first_level_key: Vec<u8>, second_level_key: Vec<u8>) -> Query {
    let mut query = Query::new_with_direction(true);
    query.insert_item(QueryItem::RangeFrom(first_level_key.clone()..));
    query.set_subquery(range_full_query());
    let mut after = Query::new_with_direction(true);
    after.insert_item(QueryItem::RangeAfter(second_level_key..));
    query.add_conditional_subquery(QueryItem::Key(first_level_key), None, Some(after));
    query
}

impl Drive {
    /// The query for one contract group's stored information: its owner, name and description.
    pub fn contract_group_info_query(contract_group_id: [u8; 32]) -> PathQuery {
        let mut query = Query::new_with_direction(true);
        query.insert_item(QueryItem::Key(CONTRACT_GROUP_INFO_KEY.to_vec()));
        PathQuery {
            path: contract_group_path_vec(&contract_group_id),
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// The query for one page of a contract group's members of one kind: at most `limit`
    /// entries in key order, continuing after the query's cursor.
    ///
    /// Whole-contract members come from `Contracts/<contract id>`, document type members from
    /// `DocumentTypes/<contract id>/<document type name>` and token members from
    /// `Tokens/<contract id>/<token position>`.
    pub fn contract_group_members_query(
        contract_group_id: [u8; 32],
        members_query: &ContractGroupMembersQuery,
        limit: u16,
    ) -> PathQuery {
        let (path, query) = match members_query {
            ContractGroupMembersQuery::Contracts { start_after } => {
                let mut query = Query::new_with_direction(true);
                match start_after {
                    None => query.insert_item(QueryItem::RangeFull(RangeFull)),
                    Some(contract_id) => {
                        query.insert_item(QueryItem::RangeAfter(contract_id.to_vec()..))
                    }
                }
                (contract_group_contracts_path_vec(&contract_group_id), query)
            }
            ContractGroupMembersQuery::DocumentTypes { start_after } => (
                contract_group_document_types_path_vec(&contract_group_id),
                match start_after {
                    None => two_level_query(),
                    Some((contract_id, document_type_name)) => two_level_query_after(
                        contract_id.to_vec(),
                        document_type_name.as_bytes().to_vec(),
                    ),
                },
            ),
            ContractGroupMembersQuery::Tokens { start_after } => (
                contract_group_tokens_path_vec(&contract_group_id),
                match start_after {
                    None => two_level_query(),
                    Some((contract_id, token_position)) => two_level_query_after(
                        contract_id.to_vec(),
                        token_position.to_be_bytes().to_vec(),
                    ),
                },
            ),
        };
        PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        }
    }

    /// The query for the contract groups a contract belongs to, as a whole, through its
    /// document types, and through its tokens. Bounded by the memberships one create transition
    /// may declare, so it needs no limit.
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
