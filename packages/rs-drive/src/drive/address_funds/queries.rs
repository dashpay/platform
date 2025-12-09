use crate::drive::Drive;
use crate::drive::RootTree;
use dpp::address_funds::PlatformAddress;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};

/// The subtree that has clear addresses
pub const CLEAR_ADDRESS_POOL: &[u8; 1] = b"c";

/// The subtree that has clear addresses
pub const CLEAR_ADDRESS_POOL_U8: u8 = b'c';

impl Drive {
    /// Path to address balance storage.
    pub fn addresses_path() -> Vec<Vec<u8>> {
        vec![vec![RootTree::AddressBalances as u8]]
    }

    /// Path to the clear-address pool under address balances.
    pub fn clear_addresses_path() -> Vec<Vec<u8>> {
        vec![
            vec![RootTree::AddressBalances as u8],
            vec![CLEAR_ADDRESS_POOL_U8],
        ]
    }

    /// The query for a single address balance and nonce.
    pub fn balance_for_clear_address_query(address: &PlatformAddress) -> PathQuery {
        let path = Self::clear_addresses_path();
        let mut path_query = PathQuery::new_single_key(path, address.to_bytes());
        path_query.query.limit = Some(1);
        path_query
    }

    /// The query for multiple address balances and nonces.
    pub fn balances_for_clear_addresses_query<'a, I>(addresses: I) -> PathQuery
    where
        I: IntoIterator<Item = &'a PlatformAddress>,
    {
        let path = Self::clear_addresses_path();
        let mut query = Query::new();
        let mut limit: u16 = 0;
        for address in addresses {
            query.insert_item(QueryItem::Key(address.to_bytes()));
            limit = limit.saturating_add(1);
        }
        PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        }
    }
}
