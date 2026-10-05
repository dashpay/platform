use crate::drive::contract_groups::paths::{
    contract_group_contracts_path_vec, contract_group_document_type_member_key,
    contract_group_document_types_path_vec, contract_group_path_vec,
    contract_group_token_member_key, contract_group_tokens_path_vec, contract_memberships_path_vec,
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
    /// `DocumentTypes/<contract id || document type name>` and token members from
    /// `Tokens/<contract id || token position>`. Every kind is one flat level, so a
    /// continuation is a plain range after the cursor's key and never descends into a subtree
    /// that might be empty (GroveDB charges such a descent against the limit).
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
                    None => range_full_query(),
                    Some((contract_id, document_type_name)) => {
                        Query::new_single_query_item(QueryItem::RangeAfter(
                            contract_group_document_type_member_key(
                                &contract_id.to_buffer(),
                                document_type_name,
                            )..,
                        ))
                    }
                },
            ),
            ContractGroupMembersQuery::Tokens { start_after } => (
                contract_group_tokens_path_vec(&contract_group_id),
                match start_after {
                    None => range_full_query(),
                    Some((contract_id, token_position)) => {
                        Query::new_single_query_item(QueryItem::RangeAfter(
                            contract_group_token_member_key(
                                &contract_id.to_buffer(),
                                *token_position,
                            )..,
                        ))
                    }
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
