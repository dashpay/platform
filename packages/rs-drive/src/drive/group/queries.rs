use crate::drive::group::paths::{
    group_action_signers_path_vec, group_closed_action_signers_path_vec, group_contract_path_vec,
    group_path_vec, ACTION_INFO_KEY, ACTION_SIGNERS_KEY, GROUP_ACTIVE_ACTIONS_KEY,
    GROUP_CLOSED_ACTIONS_KEY, GROUP_INFO_KEY,
};
use crate::drive::Drive;
use crate::query::{Query, QueryItem};
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use grovedb::{PathQuery, SizedQuery};
use std::ops::RangeFull;

impl Drive {
    /// The query for a single group info inside a contract.
    pub fn group_info_for_contract_id_and_group_contract_position_query(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
    ) -> PathQuery {
        let group_path = group_path_vec(&contract_id, group_contract_position);
        let mut path_query = PathQuery::new_single_key(group_path, GROUP_INFO_KEY.to_vec());
        path_query.query.limit = Some(1);
        path_query
    }

    /// The query for the group infos inside a contract.
    pub fn group_infos_for_contract_id_query(
        contract_id: [u8; 32],
        start_at: Option<(GroupContractPosition, bool)>,
        limit: Option<u16>,
    ) -> PathQuery {
        let group_contract_path = group_contract_path_vec(&contract_id);
        let mut query = Query::new_with_direction(true);
        if let Some((start_at, start_at_included)) = start_at {
            if start_at_included {
                query.insert_item(QueryItem::RangeFrom(start_at.to_be_bytes().to_vec()..))
            } else {
                query.insert_item(QueryItem::RangeAfter(start_at.to_be_bytes().to_vec()..))
            }
        } else {
            query.insert_item(QueryItem::RangeFull(RangeFull))
        }

        query.set_subquery_key(GROUP_INFO_KEY.to_vec());
        PathQuery {
            path: group_contract_path,
            query: SizedQuery {
                query,
                limit,
                offset: None,
            },
        }
    }

    /// Gets the active group actions
    pub fn group_action_infos_query(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_at: Option<([u8; 32], bool)>,
        limit: Option<u16>,
    ) -> PathQuery {
        let mut group_actions_path = group_path_vec(&contract_id, group_contract_position);
        match action_status {
            GroupActionStatus::ActionActive => {
                group_actions_path.push(GROUP_ACTIVE_ACTIONS_KEY.to_vec())
            }
            GroupActionStatus::ActionClosed => {
                group_actions_path.push(GROUP_CLOSED_ACTIONS_KEY.to_vec())
            }
        }
        let mut query = Query::new_with_direction(true);
        if let Some((start_at, start_at_included)) = start_at {
            if start_at_included {
                query.insert_item(QueryItem::RangeFrom(start_at.to_vec()..))
            } else {
                query.insert_item(QueryItem::RangeAfter(start_at.to_vec()..))
            }
        } else {
            query.insert_item(QueryItem::RangeFull(RangeFull))
        }

        query.set_subquery_key(ACTION_INFO_KEY.to_vec());

        PathQuery {
            path: group_actions_path,
            query: SizedQuery {
                query,
                limit,
                offset: None,
            },
        }
    }

    /// Gets the action signers query
    pub fn group_action_query(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: [u8; 32],
    ) -> PathQuery {
        let mut group_actions_path = group_path_vec(&contract_id, group_contract_position);
        match action_status {
            GroupActionStatus::ActionActive => {
                group_actions_path.push(GROUP_ACTIVE_ACTIONS_KEY.to_vec())
            }
            GroupActionStatus::ActionClosed => {
                group_actions_path.push(GROUP_CLOSED_ACTIONS_KEY.to_vec())
            }
        }
        group_actions_path.push(action_id.to_vec());
        let query = Query::new_single_key(ACTION_SIGNERS_KEY.to_vec());

        PathQuery {
            path: group_actions_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// Gets the action signers query
    pub fn group_action_signers_query(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: [u8; 32],
    ) -> PathQuery {
        let mut group_actions_path = group_path_vec(&contract_id, group_contract_position);
        match action_status {
            GroupActionStatus::ActionActive => {
                group_actions_path.push(GROUP_ACTIVE_ACTIONS_KEY.to_vec())
            }
            GroupActionStatus::ActionClosed => {
                group_actions_path.push(GROUP_CLOSED_ACTIONS_KEY.to_vec())
            }
        }
        group_actions_path.push(action_id.to_vec());
        group_actions_path.push(ACTION_SIGNERS_KEY.to_vec());
        let query = Query::new_range_full();

        PathQuery {
            path: group_actions_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// Query for a single active signer
    pub fn group_active_action_single_signer_query(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
        action_id: [u8; 32],
        identity_id: [u8; 32],
    ) -> PathQuery {
        let group_actions_path =
            group_action_signers_path_vec(&contract_id, group_contract_position, &action_id);
        let query = Query::new_single_key(identity_id.to_vec());

        PathQuery {
            path: group_actions_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// Query for a single active signer
    pub fn group_closed_action_single_signer_query(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
        action_id: [u8; 32],
        identity_id: [u8; 32],
    ) -> PathQuery {
        let group_actions_path =
            group_closed_action_signers_path_vec(&contract_id, group_contract_position, &action_id);
        let query = Query::new_single_key(identity_id.to_vec());

        PathQuery {
            path: group_actions_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// Query for a single active signer
    pub fn group_active_or_closed_action_single_signer_query(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
        action_id: [u8; 32],
        action_status: GroupActionStatus,
        identity_id: [u8; 32],
    ) -> PathQuery {
        let mut group_actions_path = group_path_vec(&contract_id, group_contract_position);
        match action_status {
            GroupActionStatus::ActionActive => {
                group_actions_path.push(GROUP_ACTIVE_ACTIONS_KEY.to_vec())
            }
            GroupActionStatus::ActionClosed => {
                group_actions_path.push(GROUP_CLOSED_ACTIONS_KEY.to_vec())
            }
        }
        group_actions_path.push(action_id.to_vec());
        group_actions_path.push(ACTION_SIGNERS_KEY.to_vec());
        let query = Query::new_single_key(identity_id.to_vec());

        PathQuery {
            path: group_actions_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// Gets query to figure out if the proof is for active or closed
    pub fn group_active_or_closed_action_query(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
    ) -> PathQuery {
        let group_path = group_path_vec(&contract_id, group_contract_position);
        let mut query = Query::new_with_direction(true);
        query.insert_keys(vec![
            GROUP_ACTIVE_ACTIONS_KEY.to_vec(),
            GROUP_CLOSED_ACTIONS_KEY.to_vec(),
        ]);

        PathQuery {
            path: group_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// Gets the signer in both the active and closed action places
    pub fn group_active_and_closed_action_single_signer_query(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
        action_id: [u8; 32],
        identity_id: [u8; 32],
    ) -> PathQuery {
        let group_path = group_path_vec(&contract_id, group_contract_position);
        let mut query = Query::new_with_direction(true);
        query.insert_keys(vec![
            GROUP_ACTIVE_ACTIONS_KEY.to_vec(),
            GROUP_CLOSED_ACTIONS_KEY.to_vec(),
        ]);
        query.set_subquery_path(vec![
            action_id.to_vec(),
            ACTION_SIGNERS_KEY.to_vec(),
            identity_id.to_vec(),
        ]);

        PathQuery {
            path: group_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTRACT_ID: [u8; 32] = [1u8; 32];
    const ACTION_ID: [u8; 32] = [2u8; 32];
    const IDENTITY_ID: [u8; 32] = [3u8; 32];
    const GROUP_POS: GroupContractPosition = 0;

    #[test]
    fn group_info_query_has_limit_of_one() {
        let pq = Drive::group_info_for_contract_id_and_group_contract_position_query(
            CONTRACT_ID,
            GROUP_POS,
        );
        assert_eq!(pq.query.limit, Some(1));
        assert_eq!(pq.path, group_path_vec(&CONTRACT_ID, GROUP_POS));
    }

    #[test]
    fn group_infos_query_with_no_start_returns_range_full() {
        let pq = Drive::group_infos_for_contract_id_query(CONTRACT_ID, None, Some(10));
        assert_eq!(pq.query.limit, Some(10));
        assert_eq!(pq.path, group_contract_path_vec(&CONTRACT_ID));
        // Should have a RangeFull query item
        assert_eq!(pq.query.query.items.len(), 1);
        assert!(
            matches!(pq.query.query.items[0], QueryItem::RangeFull(..)),
            "expected RangeFull query item"
        );
        // Should have the subquery key set to GROUP_INFO_KEY
        let subquery = pq
            .query
            .query
            .default_subquery_branch
            .subquery_path
            .as_ref();
        assert!(
            subquery.is_some(),
            "expected a subquery path for GROUP_INFO_KEY"
        );
    }

    #[test]
    fn group_infos_query_with_start_included() {
        let pq = Drive::group_infos_for_contract_id_query(CONTRACT_ID, Some((5, true)), Some(10));
        assert_eq!(pq.query.limit, Some(10));
        assert_eq!(pq.query.query.items.len(), 1);
        assert!(
            matches!(pq.query.query.items[0], QueryItem::RangeFrom(..)),
            "expected RangeFrom for included start"
        );
    }

    #[test]
    fn group_infos_query_with_start_excluded() {
        let pq = Drive::group_infos_for_contract_id_query(CONTRACT_ID, Some((5, false)), Some(10));
        assert_eq!(pq.query.limit, Some(10));
        assert_eq!(pq.query.query.items.len(), 1);
        assert!(
            matches!(pq.query.query.items[0], QueryItem::RangeAfter(..)),
            "expected RangeAfter for excluded start"
        );
    }

    #[test]
    fn action_infos_query_active_status() {
        let pq = Drive::group_action_infos_query(
            CONTRACT_ID,
            GROUP_POS,
            GroupActionStatus::ActionActive,
            None,
            Some(5),
        );
        assert_eq!(pq.query.limit, Some(5));
        // Path should end with the active actions key
        assert_eq!(pq.path.last().unwrap(), &GROUP_ACTIVE_ACTIONS_KEY.to_vec());
        // Should be a RangeFull query item when no start_at
        assert_eq!(pq.query.query.items.len(), 1);
        assert!(matches!(pq.query.query.items[0], QueryItem::RangeFull(..)));
        // Should have subquery key for ACTION_INFO_KEY
        assert!(
            pq.query
                .query
                .default_subquery_branch
                .subquery_path
                .is_some(),
            "expected a subquery path for ACTION_INFO_KEY"
        );
    }

    #[test]
    fn action_infos_query_closed_status() {
        let pq = Drive::group_action_infos_query(
            CONTRACT_ID,
            GROUP_POS,
            GroupActionStatus::ActionClosed,
            None,
            Some(5),
        );
        assert_eq!(pq.path.last().unwrap(), &GROUP_CLOSED_ACTIONS_KEY.to_vec());
    }

    #[test]
    fn action_infos_query_with_start_at_included() {
        let pq = Drive::group_action_infos_query(
            CONTRACT_ID,
            GROUP_POS,
            GroupActionStatus::ActionActive,
            Some((ACTION_ID, true)),
            Some(5),
        );
        assert_eq!(pq.query.limit, Some(5));
        assert_eq!(pq.query.query.items.len(), 1);
        assert!(
            matches!(pq.query.query.items[0], QueryItem::RangeFrom(..)),
            "expected RangeFrom for included start_at"
        );
    }

    #[test]
    fn action_infos_query_with_start_at_excluded() {
        let pq = Drive::group_action_infos_query(
            CONTRACT_ID,
            GROUP_POS,
            GroupActionStatus::ActionActive,
            Some((ACTION_ID, false)),
            Some(5),
        );
        assert_eq!(pq.query.limit, Some(5));
        assert_eq!(pq.query.query.items.len(), 1);
        assert!(
            matches!(pq.query.query.items[0], QueryItem::RangeAfter(..)),
            "expected RangeAfter for excluded start_at"
        );
    }

    #[test]
    fn group_action_query_active() {
        let pq = Drive::group_action_query(
            CONTRACT_ID,
            GROUP_POS,
            GroupActionStatus::ActionActive,
            ACTION_ID,
        );
        assert!(pq.query.limit.is_none());
        // Path includes the action_id
        assert_eq!(pq.path.last().unwrap(), &ACTION_ID.to_vec());
        // Query should contain ACTION_SIGNERS_KEY as a single key
        assert_eq!(pq.query.query.items.len(), 1);
        assert_eq!(
            pq.query.query.items[0],
            QueryItem::Key(ACTION_SIGNERS_KEY.to_vec())
        );
    }

    #[test]
    fn group_action_query_closed() {
        let pq = Drive::group_action_query(
            CONTRACT_ID,
            GROUP_POS,
            GroupActionStatus::ActionClosed,
            ACTION_ID,
        );
        // The 4th element should be the closed actions key
        assert_eq!(pq.path[3], GROUP_CLOSED_ACTIONS_KEY.to_vec());
        // Query should contain ACTION_SIGNERS_KEY as a single key
        assert_eq!(pq.query.query.items.len(), 1);
        assert_eq!(
            pq.query.query.items[0],
            QueryItem::Key(ACTION_SIGNERS_KEY.to_vec())
        );
    }

    #[test]
    fn group_action_signers_query_active() {
        let pq = Drive::group_action_signers_query(
            CONTRACT_ID,
            GROUP_POS,
            GroupActionStatus::ActionActive,
            ACTION_ID,
        );
        assert!(pq.query.limit.is_none());
        assert_eq!(pq.path.last().unwrap(), &ACTION_SIGNERS_KEY.to_vec());
        // Should be a full range query
        assert_eq!(pq.query.query.items.len(), 1);
        assert!(matches!(pq.query.query.items[0], QueryItem::RangeFull(..)));
    }

    #[test]
    fn group_action_signers_query_closed() {
        let pq = Drive::group_action_signers_query(
            CONTRACT_ID,
            GROUP_POS,
            GroupActionStatus::ActionClosed,
            ACTION_ID,
        );
        assert_eq!(pq.path[3], GROUP_CLOSED_ACTIONS_KEY.to_vec());
        assert_eq!(pq.path.last().unwrap(), &ACTION_SIGNERS_KEY.to_vec());
        // Should be a full range query
        assert!(matches!(pq.query.query.items[0], QueryItem::RangeFull(..)));
    }

    #[test]
    fn active_action_single_signer_query_path() {
        let pq = Drive::group_active_action_single_signer_query(
            CONTRACT_ID,
            GROUP_POS,
            ACTION_ID,
            IDENTITY_ID,
        );
        assert!(pq.query.limit.is_none());
        // Path should be the active signers path
        assert_eq!(
            pq.path,
            group_action_signers_path_vec(&CONTRACT_ID, GROUP_POS, &ACTION_ID)
        );
        // Query should have a single key for the identity
        assert_eq!(pq.query.query.items.len(), 1);
        assert_eq!(
            pq.query.query.items[0],
            QueryItem::Key(IDENTITY_ID.to_vec())
        );
    }

    #[test]
    fn closed_action_single_signer_query_path() {
        let pq = Drive::group_closed_action_single_signer_query(
            CONTRACT_ID,
            GROUP_POS,
            ACTION_ID,
            IDENTITY_ID,
        );
        assert!(pq.query.limit.is_none());
        assert_eq!(
            pq.path,
            group_closed_action_signers_path_vec(&CONTRACT_ID, GROUP_POS, &ACTION_ID)
        );
        // Query should have a single key for the identity
        assert_eq!(pq.query.query.items.len(), 1);
        assert_eq!(
            pq.query.query.items[0],
            QueryItem::Key(IDENTITY_ID.to_vec())
        );
    }

    #[test]
    fn active_or_closed_single_signer_query_active() {
        let pq = Drive::group_active_or_closed_action_single_signer_query(
            CONTRACT_ID,
            GROUP_POS,
            ACTION_ID,
            GroupActionStatus::ActionActive,
            IDENTITY_ID,
        );
        assert_eq!(pq.path[3], GROUP_ACTIVE_ACTIONS_KEY.to_vec());
        assert_eq!(pq.path.last().unwrap(), &ACTION_SIGNERS_KEY.to_vec());
        // Query should have a single key for the identity
        assert_eq!(pq.query.query.items.len(), 1);
        assert_eq!(
            pq.query.query.items[0],
            QueryItem::Key(IDENTITY_ID.to_vec())
        );
    }

    #[test]
    fn active_or_closed_single_signer_query_closed() {
        let pq = Drive::group_active_or_closed_action_single_signer_query(
            CONTRACT_ID,
            GROUP_POS,
            ACTION_ID,
            GroupActionStatus::ActionClosed,
            IDENTITY_ID,
        );
        assert_eq!(pq.path[3], GROUP_CLOSED_ACTIONS_KEY.to_vec());
        // Query should have a single key for the identity
        assert_eq!(pq.query.query.items.len(), 1);
        assert_eq!(
            pq.query.query.items[0],
            QueryItem::Key(IDENTITY_ID.to_vec())
        );
    }

    #[test]
    fn active_or_closed_action_query_has_both_keys() {
        let pq = Drive::group_active_or_closed_action_query(CONTRACT_ID, GROUP_POS);
        assert!(pq.query.limit.is_none());
        assert_eq!(pq.path, group_path_vec(&CONTRACT_ID, GROUP_POS));
        // Should have two keys: active and closed
        assert_eq!(pq.query.query.items.len(), 2);
        assert_eq!(
            pq.query.query.items[0],
            QueryItem::Key(GROUP_ACTIVE_ACTIONS_KEY.to_vec())
        );
        assert_eq!(
            pq.query.query.items[1],
            QueryItem::Key(GROUP_CLOSED_ACTIONS_KEY.to_vec())
        );
    }

    #[test]
    fn active_and_closed_single_signer_query_has_subquery_path() {
        let pq = Drive::group_active_and_closed_action_single_signer_query(
            CONTRACT_ID,
            GROUP_POS,
            ACTION_ID,
            IDENTITY_ID,
        );
        assert!(pq.query.limit.is_none());
        assert_eq!(pq.path, group_path_vec(&CONTRACT_ID, GROUP_POS));
        // Should have two keys: active and closed
        assert_eq!(pq.query.query.items.len(), 2);
        assert_eq!(
            pq.query.query.items[0],
            QueryItem::Key(GROUP_ACTIVE_ACTIONS_KEY.to_vec())
        );
        assert_eq!(
            pq.query.query.items[1],
            QueryItem::Key(GROUP_CLOSED_ACTIONS_KEY.to_vec())
        );
        // Should have a subquery path with action_id, signers key, and identity_id
        let subquery_path = pq
            .query
            .query
            .default_subquery_branch
            .subquery_path
            .as_ref()
            .expect("expected a subquery path");
        assert_eq!(subquery_path.len(), 3);
        assert_eq!(subquery_path[0], ACTION_ID.to_vec());
        assert_eq!(subquery_path[1], ACTION_SIGNERS_KEY.to_vec());
        assert_eq!(subquery_path[2], IDENTITY_ID.to_vec());
    }
}
