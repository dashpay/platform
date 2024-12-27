use crate::drive::tokens::token_balances_path_vec;
use crate::drive::Drive;
use crate::query::{Query, QueryItem};
use grovedb::{PathQuery, SizedQuery};
use std::ops::RangeFull;

impl Drive {
    /// The query for proving the identities balance of a token from an identity id.
    pub fn token_balance_for_identity_id_query(
        token_id: [u8; 32],
        identity_id: [u8; 32],
    ) -> PathQuery {
        let balance_path = token_balances_path_vec(token_id);
        PathQuery::new_single_key(balance_path, identity_id.to_vec())
    }

    /// The query getting a token balance for many identities
    pub fn token_balances_for_identity_ids_query(
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
    ) -> PathQuery {
        let balance_path = token_balances_path_vec(token_id);
        let mut query = Query::new();
        query.insert_keys(identity_ids.iter().map(|key| key.to_vec()).collect());
        PathQuery {
            path: balance_path,
            query: SizedQuery {
                query,
                limit: Some(identity_ids.len() as u16),
                offset: None,
            },
        }
    }

    /// The query getting token balances for identities in a range
    pub fn token_balances_for_range_query(
        token_id: [u8; 32],
        start_at: Option<([u8; 32], bool)>,
        ascending: bool,
        limit: u16,
    ) -> PathQuery {
        let balance_path = token_balances_path_vec(token_id);
        let mut query = Query::new_with_direction(ascending);
        if ascending {
            if let Some((start_at, start_at_included)) = start_at {
                if start_at_included {
                    query.insert_item(QueryItem::RangeFrom(start_at.to_vec()..))
                } else {
                    query.insert_item(QueryItem::RangeAfter(start_at.to_vec()..))
                }
            } else {
                query.insert_item(QueryItem::RangeFull(RangeFull))
            }
        } else if let Some((start_at, start_at_included)) = start_at {
            if start_at_included {
                query.insert_item(QueryItem::RangeToInclusive(..=start_at.to_vec()))
            } else {
                query.insert_item(QueryItem::RangeTo(..start_at.to_vec()))
            }
        } else {
            query.insert_item(QueryItem::RangeFull(RangeFull))
        }
        PathQuery {
            path: balance_path,
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        }
    }
}
