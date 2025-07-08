use crate::drive::tokens::paths::token_contract_infos_root_path_vec;
use crate::drive::Drive;
use grovedb::PathQuery;

impl Drive {
    /// The query for proving the contract info of a token.
    pub fn token_contract_info_query(token_id: [u8; 32]) -> PathQuery {
        let root_path = token_contract_infos_root_path_vec();
        let mut path_query = PathQuery::new_single_key(root_path, token_id.to_vec());
        path_query.query.limit = Some(1);
        path_query
    }
}
