use crate::drive::Drive;
use crate::drive::RootTree;
use dpp::identity::KeyOfType;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};

impl Drive {
    /// The query for a single address balance and nonce.
    pub fn balance_for_address_query(key_of_type: &KeyOfType) -> PathQuery {
        let path = vec![vec![RootTree::AddressBalances as u8]];
        let mut path_query = PathQuery::new_single_key(path, key_of_type.to_bytes());
        path_query.query.limit = Some(1);
        path_query
    }

    /// The query for multiple address balances and nonces.
    pub fn balances_for_addresses_query<'a, I>(keys_of_type: I) -> PathQuery
    where
        I: IntoIterator<Item = &'a KeyOfType>,
    {
        let path = vec![vec![RootTree::AddressBalances as u8]];
        let mut query = Query::new();
        for key_of_type in keys_of_type {
            query.insert_item(QueryItem::Key(key_of_type.to_bytes()));
        }
        PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }
}
