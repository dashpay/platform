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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::tokens::paths::{
        token_identity_infos_path_vec, token_identity_infos_root_path_vec,
    };

    #[test]
    fn token_info_for_identity_id_query_targets_correct_path_and_limit() {
        let token_id = [1u8; 32];
        let identity_id = [2u8; 32];

        let path_query = Drive::token_info_for_identity_id_query(token_id, identity_id);

        // Path must be the identity-infos subtree for this token.
        assert_eq!(path_query.path, token_identity_infos_path_vec(token_id));
        // Must request exactly one item.
        assert_eq!(path_query.query.limit, Some(1));
        // The underlying grovedb Query should target exactly the identity id key.
        let keys = path_query.query.query.items;
        assert_eq!(keys.len(), 1, "expected exactly one query item");
        // The item should be a Key variant whose inner bytes match identity_id.
        match &keys[0] {
            QueryItem::Key(k) => assert_eq!(k.as_slice(), identity_id.as_slice()),
            other => panic!("expected QueryItem::Key(..), got {:?}", other),
        }
    }

    #[test]
    fn token_infos_for_identity_ids_query_includes_all_keys_and_limit() {
        let token_id = [3u8; 32];
        let identity_ids = [[10u8; 32], [11u8; 32], [12u8; 32]];

        let path_query = Drive::token_infos_for_identity_ids_query(token_id, &identity_ids);

        assert_eq!(path_query.path, token_identity_infos_path_vec(token_id));
        assert_eq!(path_query.query.limit, Some(identity_ids.len() as u16));
        assert_eq!(path_query.query.offset, None);

        // Each identity id should be present as a Key item in the query.
        let items = &path_query.query.query.items;
        assert_eq!(items.len(), identity_ids.len());
        for id in identity_ids.iter() {
            assert!(
                items.iter().any(|item| match item {
                    QueryItem::Key(k) => k.as_slice() == id.as_slice(),
                    _ => false,
                }),
                "expected identity id {:?} in query items {:?}",
                id,
                items
            );
        }
    }

    #[test]
    fn token_infos_for_identity_ids_query_empty_identity_list() {
        let token_id = [4u8; 32];

        let path_query = Drive::token_infos_for_identity_ids_query(token_id, &[]);

        assert_eq!(path_query.path, token_identity_infos_path_vec(token_id));
        assert_eq!(path_query.query.limit, Some(0));
        assert!(path_query.query.query.items.is_empty());
    }

    #[test]
    fn token_infos_for_identity_id_query_uses_subquery_path() {
        let token_ids = [[5u8; 32], [6u8; 32]];
        let identity_id = [7u8; 32];

        let path_query = Drive::token_infos_for_identity_id_query(&token_ids, identity_id);

        // Root path is the overall identity-infos tree (not scoped to a token).
        assert_eq!(path_query.path, token_identity_infos_root_path_vec());
        assert_eq!(path_query.query.limit, Some(token_ids.len() as u16));

        // Default subquery path should be set to the identity id — this is what
        // makes the query descend into each token's subtree to fetch that identity.
        let expected_subquery_path = vec![identity_id.to_vec()];
        assert_eq!(
            path_query.query.query.default_subquery_branch.subquery_path,
            Some(expected_subquery_path)
        );

        // Each token id must appear as a Key item in the top-level query.
        let items = &path_query.query.query.items;
        assert_eq!(items.len(), token_ids.len());
        for id in token_ids.iter() {
            assert!(
                items.iter().any(|item| match item {
                    QueryItem::Key(k) => k.as_slice() == id.as_slice(),
                    _ => false,
                }),
                "expected token id {:?} in query items {:?}",
                id,
                items
            );
        }
    }

    #[test]
    fn token_infos_for_range_query_ascending_with_no_start_uses_range_full() {
        let token_id = [8u8; 32];
        let path_query = Drive::token_infos_for_range_query(token_id, None, true, 50);

        assert_eq!(path_query.path, token_identity_infos_path_vec(token_id));
        assert_eq!(path_query.query.limit, Some(50));
        assert!(
            path_query.query.query.left_to_right,
            "expected ascending query direction when ascending=true"
        );

        let items = &path_query.query.query.items;
        assert_eq!(items.len(), 1);
        assert!(
            matches!(&items[0], QueryItem::RangeFull(_)),
            "expected RangeFull when no start_at is provided, got {:?}",
            items[0]
        );
    }

    #[test]
    fn token_infos_for_range_query_ascending_with_start_inclusive_uses_range_from() {
        let token_id = [9u8; 32];
        let start_at = [1u8; 32];
        let path_query =
            Drive::token_infos_for_range_query(token_id, Some((start_at, true)), true, 10);

        let items = &path_query.query.query.items;
        assert_eq!(items.len(), 1);
        match &items[0] {
            QueryItem::RangeFrom(r) => assert_eq!(r.start.as_slice(), start_at.as_slice()),
            other => panic!(
                "expected RangeFrom when ascending+inclusive, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn token_infos_for_range_query_ascending_with_start_exclusive_uses_range_after() {
        let token_id = [10u8; 32];
        let start_at = [2u8; 32];
        let path_query =
            Drive::token_infos_for_range_query(token_id, Some((start_at, false)), true, 10);

        let items = &path_query.query.query.items;
        assert_eq!(items.len(), 1);
        match &items[0] {
            QueryItem::RangeAfter(r) => assert_eq!(r.start.as_slice(), start_at.as_slice()),
            other => panic!(
                "expected RangeAfter when ascending+exclusive, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn token_infos_for_range_query_descending_with_start_inclusive_uses_range_to_inclusive() {
        let token_id = [11u8; 32];
        let start_at = [3u8; 32];
        let path_query =
            Drive::token_infos_for_range_query(token_id, Some((start_at, true)), false, 5);

        let items = &path_query.query.query.items;
        assert_eq!(items.len(), 1);
        match &items[0] {
            QueryItem::RangeToInclusive(r) => assert_eq!(r.end.as_slice(), start_at.as_slice()),
            other => panic!(
                "expected RangeToInclusive when descending+inclusive, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn token_infos_for_range_query_descending_with_start_exclusive_uses_range_to() {
        let token_id = [12u8; 32];
        let start_at = [4u8; 32];
        let path_query =
            Drive::token_infos_for_range_query(token_id, Some((start_at, false)), false, 5);

        let items = &path_query.query.query.items;
        assert_eq!(items.len(), 1);
        match &items[0] {
            QueryItem::RangeTo(r) => assert_eq!(r.end.as_slice(), start_at.as_slice()),
            other => panic!(
                "expected RangeTo when descending+exclusive, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn token_infos_for_range_query_descending_without_start_uses_range_full() {
        let token_id = [13u8; 32];
        let path_query = Drive::token_infos_for_range_query(token_id, None, false, 100);

        assert_eq!(path_query.query.limit, Some(100));
        assert!(
            !path_query.query.query.left_to_right,
            "expected descending query direction when ascending=false"
        );
        let items = &path_query.query.query.items;
        assert_eq!(items.len(), 1);
        assert!(
            matches!(&items[0], QueryItem::RangeFull(_)),
            "expected RangeFull when descending with no start_at, got {:?}",
            items[0]
        );
    }
}
