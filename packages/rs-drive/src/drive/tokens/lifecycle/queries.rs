use crate::drive::tokens::paths::{
    token_contract_infos_root_path_vec, token_contract_lifecycles_root_path_vec,
    TOKEN_DESTROYED_SUPPLY_KEY,
};
use crate::drive::Drive;
use crate::query::Query;
use grovedb::{PathQuery, SizedQuery};

/// The limit of a query over `key_count` keys. A `SizedQuery` limit is a `u16`; a count it
/// cannot express falls back to unlimited rather than a truncated limit that would drop
/// keys from the read.
fn key_count_limit(key_count: usize) -> Option<u16> {
    u16::try_from(key_count).ok()
}

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
            SizedQuery::new(query, key_count_limit(contract_ids.len()), None),
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
            SizedQuery::new(query, key_count_limit(token_ids.len()), None),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_limit_a_merged_query_to_its_key_count_or_not_at_all() {
        let few: Vec<[u8; 32]> = (0..3u8).map(|i| [i; 32]).collect();
        assert_eq!(
            Drive::contract_token_lifecycles_query(&few).query.limit,
            Some(3)
        );
        assert_eq!(Drive::token_contract_infos_query(&few).query.limit, Some(3));

        assert_eq!(key_count_limit(u16::MAX as usize), Some(u16::MAX));
        assert_eq!(key_count_limit(u16::MAX as usize + 1), None);
    }
}
