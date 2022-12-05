use crate::drive::flags::StorageFlags;
use crate::drive::identity::IDENTITY_KEY;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::identity::Identity;
use crate::query::{Query, QueryItem};
use grovedb::query_result_type::QueryResultType::QueryElementResultType;
use grovedb::{Element, PathQuery, SizedQuery, TransactionArg};

impl Drive {
    /// Given an identity, fetches the identity with its flags from storage.
    pub fn fetch_identity(
        &self,
        id: &[u8],
        transaction: TransactionArg,
    ) -> Result<(Identity, Option<StorageFlags>), Error> {
        // get element from GroveDB
        let element = self
            .grove
            .get(
                [Into::<&[u8; 1]>::into(RootTree::Identities).as_slice(), id],
                &IDENTITY_KEY,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        // extract identity from element and deserialize the identity
        if let Element::Item(identity_cbor, element_flags) = &element {
            let identity = Identity::from_buffer(identity_cbor.as_slice()).map_err(|_| {
                Error::Identity(IdentityError::IdentitySerialization(
                    "failed to de-serialize identity from CBOR",
                ))
            })?;

            Ok((
                identity,
                StorageFlags::from_some_element_flags_ref(element_flags)?,
            ))
        } else {
            Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                "identity must be an item",
            )))
        }
    }
    /// Given a vector of identities, fetches the identities from storage.
    pub fn fetch_identities(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<Vec<Identity>, Error> {
        Ok(self
            .fetch_identities_with_flags(ids, transaction)?
            .into_iter()
            .map(|(identity, _)| identity)
            .collect())
    }

    /// Given a vector of identities, fetches the identities with their flags from storage.
    pub fn fetch_identities_with_flags(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<Vec<(Identity, Option<StorageFlags>)>, Error> {
        let mut query = Query::new();
        query.set_subquery_key(IDENTITY_KEY.to_vec());
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
            .query_raw(&path_query, QueryElementResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        result_items
            .to_elements()
            .into_iter()
            .map(|element| {
                if let Element::Item(identity_cbor, element_flags) = &element {
                    let identity =
                        Identity::from_buffer(identity_cbor.as_slice()).map_err(|_| {
                            Error::Identity(IdentityError::IdentitySerialization(
                                "failed to de-serialize identity from CBOR",
                            ))
                        })?;

                    Ok((
                        identity,
                        StorageFlags::from_some_element_flags_ref(element_flags)?,
                    ))
                } else {
                    Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                        "identity must be an item",
                    )))
                }
            })
            .collect()
    }
}
