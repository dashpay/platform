use crate::drive::balances::total_tokens_root_supply_path_vec;
use crate::drive::tokens::paths::{token_balances_path_vec, token_balances_root_path_vec};
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use grovedb::{PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;
use std::ops::RangeFull;

impl Drive {
    /// The query for proving the identities balance of a token from an identity id.
    pub fn token_balance_for_identity_id_query(
        token_id: [u8; 32],
        identity_id: [u8; 32],
    ) -> PathQuery {
        let balance_path = token_balances_path_vec(token_id);
        let mut path_query = PathQuery::new_single_key(balance_path, identity_id.to_vec());
        path_query.query.limit = Some(1);
        path_query
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

    /// The query getting token balances for a single identity and many tokens
    pub fn token_balances_for_identity_id_query(
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
    ) -> PathQuery {
        let tokens_root = token_balances_root_path_vec();

        let mut query = Query::new();

        for token_id in token_ids {
            query.insert_key(token_id.to_vec());
        }

        query.set_subquery_path(vec![identity_id.to_vec()]);

        PathQuery::new(
            tokens_root,
            SizedQuery::new(query, Some(token_ids.len() as u16), None),
        )
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

    /// The query getting token balances for identities in a range
    pub fn token_total_supply_and_aggregated_identity_balances_query(
        token_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let path_holding_total_token_supply = total_tokens_root_supply_path_vec();
        let token_supply_query =
            PathQuery::new_single_key(path_holding_total_token_supply, token_id.to_vec());
        let tokens_root_path = token_balances_root_path_vec();
        let token_aggregated_identity_balances_query =
            PathQuery::new_single_key(tokens_root_path, token_id.to_vec());
        let mut path_query = PathQuery::merge(
            vec![
                &token_aggregated_identity_balances_query,
                &token_supply_query,
            ],
            &platform_version.drive.grove_version,
        )?;
        path_query.query.limit = Some(2);
        Ok(path_query)
    }
}
