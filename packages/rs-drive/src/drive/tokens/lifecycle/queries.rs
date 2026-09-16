use crate::drive::tokens::paths::{
    token_contract_infos_root_path_vec, token_contract_lifecycles_root_path_vec,
    TOKEN_DESTROYED_SUPPLY_KEY,
};
use crate::drive::Drive;
use crate::query::Query;
use grovedb::{PathQuery, SizedQuery};

impl Drive {
    /// The query for one contract's token lifecycle record.
    pub fn contract_token_lifecycle_query(contract_id: [u8; 32]) -> PathQuery {
        let mut path_query = PathQuery::new_single_key(
            token_contract_lifecycles_root_path_vec(),
            contract_id.to_vec(),
        );
        path_query.query.limit = Some(1);
        path_query
    }

    /// The query for the token lifecycle records of several contracts.
    pub fn contract_token_lifecycles_query(contract_ids: &[[u8; 32]]) -> PathQuery {
        let mut query = Query::new();
        for contract_id in contract_ids {
            query.insert_key(contract_id.to_vec());
        }
        PathQuery::new(
            token_contract_lifecycles_root_path_vec(),
            SizedQuery::new(query, Some(contract_ids.len() as u16), None),
        )
    }

    /// The query for the contract info leaves of several tokens, which is how a token id
    /// resolves to its issuer.
    pub fn token_contract_infos_query(token_ids: &[[u8; 32]]) -> PathQuery {
        let mut query = Query::new();
        for token_id in token_ids {
            query.insert_key(token_id.to_vec());
        }
        PathQuery::new(
            token_contract_infos_root_path_vec(),
            SizedQuery::new(query, Some(token_ids.len() as u16), None),
        )
    }

    /// The query for the destroyed supply scalar.
    pub fn token_destroyed_supply_query() -> PathQuery {
        let mut path_query = PathQuery::new_single_key(
            token_contract_lifecycles_root_path_vec(),
            TOKEN_DESTROYED_SUPPLY_KEY.to_vec(),
        );
        path_query.query.limit = Some(1);
        path_query
    }
}
