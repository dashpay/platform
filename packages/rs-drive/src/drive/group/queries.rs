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
