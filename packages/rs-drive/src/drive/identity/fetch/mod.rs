#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::identity::identity_path_vec;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;

#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::Drive;
#[cfg(feature = "server")]
use crate::drive::RootTree;
#[cfg(feature = "server")]
use crate::error::drive::DriveError;

#[cfg(feature = "server")]
use crate::error::Error;

#[cfg(any(feature = "server", feature = "verify"))]
use crate::query::Query;
#[cfg(feature = "server")]
use crate::query::QueryItem;

#[cfg(feature = "server")]
use grovedb::query_result_type::QueryResultType::{
    QueryElementResultType, QueryKeyElementPairResultType,
};
#[cfg(feature = "server")]
use grovedb::Element::SumItem;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
#[cfg(any(feature = "server", feature = "verify"))]
use grovedb::{PathQuery, SizedQuery};

#[cfg(feature = "server")]
use dpp::fee::Credits;

#[cfg(feature = "server")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "server")]
use std::collections::BTreeMap;

#[cfg(feature = "server")]
mod balance;
#[cfg(feature = "server")]
mod contract_keys;
#[cfg(feature = "server")]
mod fetch_by_public_key_hashes;
#[cfg(feature = "server")]
mod full_identity;
#[cfg(feature = "server")]
mod nonce;
#[cfg(feature = "server")]
mod partial_identity;
#[cfg(feature = "server")]
mod prove;
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) mod queries;
#[cfg(feature = "server")]
mod revision;

impl Drive {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// The query for the identity revision
    pub fn identity_revision_query(identity_id: &[u8; 32]) -> PathQuery {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let mut query = Query::new();
        query.insert_key(vec![IdentityTreeRevision as u8]);
        PathQuery {
            path: identity_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    #[cfg(feature = "server")]
    /// Given a vector of identities, fetches the identities from storage.
    pub fn verify_all_identities_exist(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let mut query = Query::new();
        for id in ids {
            query.insert_item(QueryItem::Key(id.to_vec()));
        }
        let path_query = PathQuery {
            path: vec![vec![RootTree::Identities as u8]],
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };
        let (result_items, _) = self
            .grove
            .query_raw(
                &path_query,
                true,
                true,
                true,
                QueryElementResultType,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        Ok(result_items.len() == ids.len())
    }

    #[cfg(feature = "server")]
    /// Given a vector of identities, fetches the identities from storage.
    pub fn fetch_identities_balances(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], u64>, Error> {
        let mut query = Query::new();
        for id in ids {
            query.insert_item(QueryItem::Key(id.to_vec()));
        }
        let path_query = PathQuery {
            path: vec![vec![RootTree::Balances as u8]],
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };
        let (result_items, _) = self
            .grove
            .query_raw(
                &path_query,
                true,
                true,
                true,
                QueryKeyElementPairResultType,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        result_items
            .to_key_elements()
            .into_iter()
            .map(|key_element| {
                if let SumItem(balance, _) = &key_element.1 {
                    let identifier: [u8; 32] = key_element.0.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(String::from(
                            "expected 32 bytes",
                        )))
                    })?;
                    Ok((identifier, *balance as u64))
                } else {
                    Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                        "identity balance must be a sum item",
                    )))
                }
            })
            .collect()
    }

    #[cfg(feature = "server")]
    /// Given a vector of identities, fetches the identities from storage.
    pub fn fetch_optional_identities_balances(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<Credits>>, Error> {
        let mut query = Query::new();
        for id in ids {
            query.insert_item(QueryItem::Key(id.to_vec()));
        }
        let path_query = PathQuery {
            path: vec![vec![RootTree::Balances as u8]],
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };
        let results = self
            .grove
            .query_raw_keys_optional(
                &path_query,
                true,
                true,
                true,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        results
            .into_iter()
            .map(|(_, key, element)| {
                let identifier: [u8; 32] = key.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(String::from(
                        "expected 32 bytes",
                    )))
                })?;

                let balance = element
                    .map(|element| {
                        if let SumItem(balance, _) = &element {
                            Ok(*balance as u64)
                        } else {
                            Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                                "identity balance must be a sum item",
                            )))
                        }
                    })
                    .transpose()?;

                Ok((identifier, balance))
            })
            .collect()
    }

    #[cfg(feature = "server")]
    /// Given a vector of identities, fetches the identities from storage.
    pub fn fetch_many_identity_balances_by_range<I>(
        &self,
        start_at: Option<([u8; 32], bool)>,
        ascending: bool,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<I, Error>
    where
        I: FromIterator<([u8; 32], u64)>,
    {
        let balance_query = Self::balances_for_range_query(start_at, ascending, limit);
        let (result_items, _) = self
            .grove
            .query_raw(
                &balance_query,
                true,
                true,
                true,
                QueryKeyElementPairResultType,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        result_items
            .to_key_elements()
            .into_iter()
            .map(|key_element| {
                if let SumItem(balance, _) = &key_element.1 {
                    let identifier: [u8; 32] = key_element.0.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(String::from(
                            "expected 32 bytes",
                        )))
                    })?;
                    Ok((identifier, *balance as u64))
                } else {
                    Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                        "identity balance must be a sum item",
                    )))
                }
            })
            .collect()
    }
}
