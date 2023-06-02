use crate::common::encode::encode_u64;
use crate::drive::contract::paths::{
    contract_keeping_history_storage_path_vec, contract_root_path_vec,
};
use crate::drive::contract::{paths, MAX_CONTRACT_HISTORY_FETCH_LIMIT};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::error::Error::GroveDB;
use crate::query::{Query, QueryItem};
use grovedb::{PathQuery, SizedQuery};

impl Drive {
    /// Creates a path query for a specified contract.
    ///
    /// This function takes a contract ID and creates a path query for fetching the contract data.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   create the path query.
    ///
    /// # Returns
    ///
    /// * `PathQuery` - A `PathQuery` object representing the query for fetching the contract data.
    pub fn fetch_contract_query(contract_id: [u8; 32]) -> PathQuery {
        let contract_path = contract_root_path_vec(contract_id.as_slice());
        PathQuery::new_single_key(contract_path, vec![0])
    }

    /// Creates a path query for a specified contract.
    ///
    /// This function takes a contract ID and creates a path query for fetching the contract data.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   create the path query.
    ///
    /// # Returns
    ///
    /// * `PathQuery` - A `PathQuery` object representing the query for fetching the contract data.
    pub fn fetch_contract_with_history_latest_query(contract_id: [u8; 32]) -> PathQuery {
        let contract_path = contract_keeping_history_storage_path_vec(contract_id.as_slice());
        PathQuery::new_single_key(contract_path, vec![0])
    }

    /// Creates a merged path query for multiple contracts.
    ///
    /// This function takes a slice of contract IDs and creates a merged path query for fetching
    /// the data of all specified contracts.
    ///
    /// # Arguments
    ///
    /// * `contract_ids` - A slice of contract IDs as 32-byte arrays. The contract IDs are used to
    ///   create the path queries.
    ///
    /// # Returns
    ///
    /// * `Result<PathQuery, Error>` - If successful, returns a `PathQuery` object representing the
    ///   merged query for fetching the contracts' data. If an error occurs during the merging of
    ///   path queries, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the merging of path queries fails.
    pub fn fetch_contracts_query(contract_ids: &[[u8; 32]]) -> Result<PathQuery, Error> {
        let mut queries = Vec::new();
        for contract_id in contract_ids {
            queries.push(Self::fetch_contract_query(*contract_id));
        }
        PathQuery::merge(queries.iter().collect()).map_err(GroveDB)
    }

    /// Creates a path query for historical entries of a specified contract.
    ///
    /// This function takes a slice of contract IDs and creates a path query for fetching
    /// the historical data of the specified contract.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   create the path query.
    ///
    /// # Returns
    ///
    /// * `Result<PathQuery, Error>` - If successful, returns a `PathQuery` object representing the
    ///   merged query for fetching the contracts' data. If limit are outside of the
    ///   allowed range, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the limit is out of the allowed range.
    pub fn fetch_contract_history_query(
        contract_id: [u8; 32],
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
    ) -> Result<PathQuery, Error> {
        let limit = limit.unwrap_or(MAX_CONTRACT_HISTORY_FETCH_LIMIT);
        if !(1..=MAX_CONTRACT_HISTORY_FETCH_LIMIT).contains(&limit) {
            return Err(Error::Drive(DriveError::InvalidContractHistoryFetchLimit(
                limit,
            )));
        }

        let query = Query::new_single_query_item_with_direction(
            QueryItem::RangeAfter(std::ops::RangeFrom {
                start: encode_u64(start_at_date),
            }),
            false,
        );

        Ok(PathQuery::new(
            paths::contract_keeping_history_storage_path_vec(&contract_id),
            SizedQuery::new(query, Some(limit), offset),
        ))
    }
}
