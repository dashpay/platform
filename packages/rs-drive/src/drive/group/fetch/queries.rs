use crate::drive::group::paths::{group_contract_path_vec, group_path_vec, GROUP_INFO_KEY};
use crate::drive::Drive;
use crate::query::{Query, QueryItem};
use dpp::data_contract::GroupContractPosition;
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
}
