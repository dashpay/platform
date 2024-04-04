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
use std::collections::BTreeMap;

#[cfg(feature = "server")]
mod balance;
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
            .query_raw(&path_query, true, true, QueryElementResultType, transaction)
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
                QueryKeyElementPairResultType,
                transaction,
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

    // TODO: We deal with it in an upcoming PR (Sam!!)
    // /// Given a vector of identities, fetches the identities with their keys
    // /// matching the request from storage.
    // pub fn fetch_identities_with_keys(
    //     &self,
    //     ids: Vec<[u8; 32]>,
    //     key_ref_request: KeyRequestType,
    //     transaction: TransactionArg,
    // ) -> Result<Vec<Identity>, Error> {
    //     let key_request = IdentityKeysRequest {
    //         identity_id: [],
    //         key_request: KeyRequestType::AllKeysRequest,
    //         limit: None,
    //         offset: None,
    //     }
    //     let mut query = Query::new();
    //     query.set_subquery_key(IDENTITY_KEY.to_vec());
    //
    //     let (result_items, _) = self
    //         .grove
    //         .query_raw(&path_query, QueryElementResultType, transaction)
    //         .unwrap()
    //         .map_err(Error::GroveDB)?;
    //
    //     result_items
    //         .to_elements()
    //         .into_iter()
    //         .map(|element| {
    //             if let Element::Item(identity_cbor, element_flags) = &element {
    //                 let identity =
    //                     Identity::from_buffer(identity_cbor.as_slice()).map_err(|_| {
    //                         Error::Identity(IdentityError::IdentitySerialization(
    //                             "failed to deserialize an identity",
    //                         ))
    //                     })?;
    //
    //                 Ok((
    //                     identity,
    //                     StorageFlags::from_some_element_flags_ref(element_flags)?,
    //                 ))
    //             } else {
    //                 Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
    //                     "identity must be an item",
    //                 )))
    //             }
    //         })
    //         .collect()
    // }
}
