use crate::drive::contract::moderation::types::ContractModerationEntriesQuery;
use crate::drive::contract::paths::contract_moderation_list_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use dpp::data_contract::config::moderation::ContractModerationList;
use grovedb::{PathQuery, SizedQuery};
use grovedb_version::version::GroveVersion;
use std::ops::RangeFull;

impl Drive {
    /// The query for one identity's entry in one moderation list of a contract: the key is
    /// proved present with its value, or absent.
    pub fn contract_moderation_entry_query(
        contract_id: [u8; 32],
        list: ContractModerationList,
        identity_id: [u8; 32],
    ) -> PathQuery {
        let mut query = Query::new_with_direction(true);
        query.insert_item(QueryItem::Key(identity_id.to_vec()));
        PathQuery {
            path: contract_moderation_list_path_vec(&contract_id, list),
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// The query for one identity's status on a contract: its entry in every list of `lists`,
    /// merged into one proof. `lists` are the lists the contract's config declares; an
    /// undeclared list has no tree and cannot be queried.
    pub fn contract_moderation_status_query(
        contract_id: [u8; 32],
        identity_id: [u8; 32],
        lists: &[ContractModerationList],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let queries: Vec<PathQuery> = lists
            .iter()
            .map(|list| Self::contract_moderation_entry_query(contract_id, *list, identity_id))
            .collect();
        match queries.as_slice() {
            [single] => Ok(single.clone()),
            _ => Ok(PathQuery::merge(queries.iter().collect(), grove_version)?),
        }
    }

    /// The query for one page of one moderation list of a contract: at most `limit` entries
    /// in identity id order, continuing after the cursor.
    pub fn contract_moderation_entries_query(
        contract_id: [u8; 32],
        entries_query: &ContractModerationEntriesQuery,
    ) -> PathQuery {
        let mut query = Query::new_with_direction(true);
        match entries_query.start_after {
            None => query.insert_item(QueryItem::RangeFull(RangeFull)),
            Some(identity_id) => query.insert_item(QueryItem::RangeAfter(identity_id.to_vec()..)),
        }
        PathQuery {
            path: contract_moderation_list_path_vec(&contract_id, entries_query.list),
            query: SizedQuery {
                query,
                limit: Some(entries_query.limit),
                offset: None,
            },
        }
    }
}
