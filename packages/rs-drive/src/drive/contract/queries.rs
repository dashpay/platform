use crate::drive::contract::paths::{contract_storage_path_vec};
use crate::drive::Drive;
use grovedb::PathQuery;

impl Drive {
    /// The query for proving a contract from a contract id.
    pub fn fetch_contract_query(contract_id: [u8; 32]) -> PathQuery {
        let contract_path = contract_storage_path_vec(contract_id.as_slice());
        PathQuery::new_single_key(contract_path, contract_id.to_vec())
    }
}
