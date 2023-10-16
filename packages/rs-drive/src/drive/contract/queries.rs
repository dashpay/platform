use crate::common::encode::encode_u64;
use crate::drive::contract::paths::{
    contract_keeping_history_storage_path_vec, contract_root_path_vec,
};
use crate::drive::contract::{paths, MAX_CONTRACT_HISTORY_FETCH_LIMIT};
use crate::drive::{Drive, RootTree};
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
    /// Note it can only be used for simple queries that are not merged with other queries.
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
        let mut query = PathQuery::new_single_key(contract_path, vec![0]);

        // TODO: remove this limit once `verify_query_with_absence_proof` supports queries without limits
        query.query.limit = Some(1);
        query
    }

    /// Creates a path query for a specified contract.
    ///
    /// This function takes a contract ID and creates a path query for fetching the contract data.
    ///
    /// Note it can only be used for simple queries that are not merged with other queries.
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
        let mut query = PathQuery::new_single_key(contract_path, vec![0]);

        // TODO: remove this limit once `verify_query_with_absence_proof` supports queries without limits
        query.query.limit = Some(1);

        query
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
    pub fn fetch_non_historical_contracts_query(contract_ids: &[[u8; 32]]) -> PathQuery {
        let mut query = Query::new();
        query.insert_keys(contract_ids.iter().map(|key| key.to_vec()).collect());
        query.set_subquery_key(vec![0]);
        PathQuery::new(
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()],
            SizedQuery::new(query, Some(contract_ids.len() as u16), None),
        )
    }

    /// Creates a merged path query for multiple contracts.
    ///
    /// This function takes a slice of contract IDs and creates a merged path query for fetching
    /// the data of all specified contracts.
    ///
    /// # Arguments
    ///
    /// * `non_historical_contract_ids` - A slice of contract IDs as 32-byte arrays. The contract IDs are used to
    ///   create the path queries.
    /// * `historical_contract_ids` - A slice of contract IDs as 32-byte arrays. The contract IDs are used to
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
    pub fn fetch_contracts_query(
        non_historical_contract_ids: &[[u8; 32]],
        historical_contract_ids: &[[u8; 32]],
    ) -> Result<PathQuery, Error> {
        if non_historical_contract_ids.is_empty() {
            return Ok(Self::fetch_historical_contracts_query(
                historical_contract_ids,
            ));
        }
        if historical_contract_ids.is_empty() {
            return Ok(Self::fetch_non_historical_contracts_query(
                non_historical_contract_ids,
            ));
        }
        let mut contracts_query =
            Self::fetch_non_historical_contracts_query(non_historical_contract_ids);
        contracts_query.query.limit = None;
        let mut historical_contracts_query =
            Self::fetch_historical_contracts_query(historical_contract_ids);
        historical_contracts_query.query.limit = None;
        PathQuery::merge(vec![&contracts_query, &historical_contracts_query]).map_err(GroveDB)
    }

    /// Creates a merged path query for multiple historical contracts.
    ///
    /// This function takes a slice of contract IDs and creates a merged path query for fetching
    /// the data of all specified contracts.
    ///
    /// # Arguments
    ///
    /// * `historical_contract_ids` - A slice of contract IDs as 32-byte arrays. The contract IDs are used to
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
    pub fn fetch_historical_contracts_query(historical_contract_ids: &[[u8; 32]]) -> PathQuery {
        let mut query = Query::new();
        query.insert_keys(
            historical_contract_ids
                .iter()
                .map(|key| key.to_vec())
                .collect(),
        );
        query.set_subquery_path(vec![vec![0], vec![0]]);
        PathQuery::new(
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()],
            SizedQuery::new(query, Some(historical_contract_ids.len() as u16), None),
        )
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
        start_at_ms: u64,
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
                start: encode_u64(start_at_ms),
            }),
            false,
        );

        Ok(PathQuery::new(
            paths::contract_keeping_history_storage_path_vec(&contract_id),
            SizedQuery::new(query, Some(limit), offset),
        ))
    }
}
