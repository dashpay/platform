use crate::common::encode::encode_u64;
use crate::drive::contract::paths::{contract_root_path_vec, contract_storage_path_vec};
use crate::drive::Drive;
use crate::error::Error;
use crate::error::Error::GroveDB;
use grovedb::PathQuery;

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
        PathQuery::new_single_key(contract_path, encode_u64(0).unwrap())
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
}
