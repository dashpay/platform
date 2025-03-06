use crate::drive::tokens::paths::{
    token_identity_infos_path_vec, token_identity_infos_root_path_vec,
};
use crate::drive::Drive;
use crate::query::{Query, QueryItem};
use grovedb::{PathQuery, SizedQuery};
use std::ops::RangeFull;

impl Drive {
    /// The query for proving the identities info of a token from an identity id.
    pub fn token_info_for_identity_id_query(
        token_id: [u8; 32],
        identity_id: [u8; 32],
    ) -> PathQuery {
        let info_path = token_identity_infos_path_vec(token_id);
        let mut path_query = PathQuery::new_single_key(info_path, identity_id.to_vec());
        path_query.query.limit = Some(1);
        path_query
    }

    /// The query getting a token info for many identities
    pub fn token_infos_for_identity_ids_query(
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
    ) -> PathQuery {
        let info_path = token_identity_infos_path_vec(token_id);
        let mut query = Query::new();
        query.insert_keys(identity_ids.iter().map(|key| key.to_vec()).collect());
        PathQuery {
            path: info_path,
            query: SizedQuery {
                query,
                limit: Some(identity_ids.len() as u16),
                offset: None,
            },
        }
    }

    /// The query getting a token infos for one identity
    pub fn token_infos_for_identity_id_query(
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
    ) -> PathQuery {
        let tokens_root = token_identity_infos_root_path_vec();

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

    /// The query getting token infos for identities in a range
    pub fn token_infos_for_range_query(
        token_id: [u8; 32],
        start_at: Option<([u8; 32], bool)>,
        ascending: bool,
        limit: u16,
    ) -> PathQuery {
        let info_path = token_identity_infos_path_vec(token_id);
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
            path: info_path,
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        }
    }
}
