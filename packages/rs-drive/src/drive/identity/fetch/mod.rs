use crate::drive::block_info::BlockInfo;
use crate::drive::defaults::PROTOCOL_VERSION;

use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
use crate::drive::identity::{identity_path, identity_path_vec};

use crate::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap};
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;

use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use crate::query::{Query, QueryItem};
use dpp::identifier::Identifier;
use dpp::identity::Identity;

use crate::drive::balances::{balance_path, balance_path_vec};
use crate::fee::credits::Credits;
use crate::fee::result::FeeResult;
use crate::fee_pools::epochs::Epoch;
use grovedb::query_result_type::QueryResultType::{
    QueryElementResultType, QueryKeyElementPairResultType,
};
use grovedb::Element::{Item, SumItem};
use grovedb::{PathQuery, SizedQuery, TransactionArg};
use integer_encoding::VarInt;
use std::collections::BTreeMap;

mod balance;
mod fetch_by_public_key_hashes;
mod full_identity;
mod revision;

impl Drive {
    /// The query for the identity revision
    pub fn identity_revision_query(identity_id: [u8; 32]) -> PathQuery {
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
            .query_raw(&path_query, true, QueryElementResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        Ok(result_items.len() == ids.len())
    }

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
                        Error::Drive(DriveError::CorruptedSerialization("expected 32 bytes"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    use grovedb::GroveDb;

    mod fetch_full_identity {
        use super::*;

        #[test]
        fn should_return_none_if_identity_is_not_present() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = drive
                .fetch_full_identity(
                    [
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0,
                    ],
                    None,
                )
                .expect("should return none");

            assert!(identity.is_none());
        }
    }

    mod fetch_proved_full_identity {
        use super::*;
        use grovedb::query_result_type::QueryResultType;
        use std::ops::RangeFull;

        #[test]
        fn test_full_identity_query_construction() {
            let identity = Identity::random_identity(5, Some(12345));
            let query = Drive::full_identity_query(identity.id.to_buffer())
                .expect("expected to make the query");
        }
        #[test]
        fn test_proved_full_identity_query_no_tx() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            drive
                .add_new_identity(identity.clone(), &BlockInfo::default(), true, None)
                .expect("expected to insert identity");

            let path_query = Drive::full_identity_query(identity.id.to_buffer())
                .expect("expected to make the query");

            // The query is querying
            //                     root
            //            /                     \
            //     identities                     balances
            //      /                                        \
            //                                             identity_id

            let query = path_query.query.query.clone();
            assert_eq!(path_query.path, Vec::<Vec<u8>>::new()); // it splits at the root
            assert_eq!(query.items.len(), 2); // 32 (Identities) and 96 (Balances)

            let conditional_subqueries = query
                .conditional_subquery_branches
                .expect("expected to have conditional subqueries");

            assert_eq!(conditional_subqueries.len(), 2); // 32 (Identities) and 96 (Balances)

            let (_, identity_conditional_subquery) = conditional_subqueries.last().unwrap();
            let (_, balance_conditional_subquery) = conditional_subqueries.first().unwrap();

            // Lets start with balance
            // There should be no subquery path

            assert_eq!(balance_conditional_subquery.subquery_path, None);

            // There should be a subquery pointing to the identity id

            let balance_subquery = *balance_conditional_subquery
                .subquery
                .clone()
                .expect("expected balances to have a subquery");

            assert_eq!(balance_subquery.conditional_subquery_branches, None);

            assert_eq!(balance_subquery.items.len(), 1);
            assert_eq!(
                balance_subquery.items.first().unwrap(),
                &QueryItem::Key(identity.id.to_buffer().to_vec())
            );

            // Moving on to Identity subquery

            // The subquery path is our identity
            assert_eq!(
                identity_conditional_subquery.subquery_path,
                Some(vec![identity.id.to_buffer().to_vec()])
            );

            let identity_subquery = *identity_conditional_subquery
                .subquery
                .clone()
                .expect("expected identities to have a subquery");

            assert_eq!(identity_subquery.items.len(), 2); // This query is for the Revision 0 and Keys 1

            let identity_conditional_subqueries = identity_subquery
                .conditional_subquery_branches
                .expect("expected to have conditional subqueries");

            // We only subquery the keys
            assert_eq!(identity_conditional_subqueries.len(), 1);

            let (_, identity_keys_conditional_subquery) =
                identity_conditional_subqueries.first().unwrap();

            assert_eq!(identity_keys_conditional_subquery.subquery_path, None);

            // We are requesting all keys
            //todo: maybe we shouldn't be

            let identity_keys_subquery = *identity_keys_conditional_subquery
                .subquery
                .clone()
                .expect("expected identities to have a subquery");

            assert_eq!(
                identity_keys_subquery.items.first().unwrap(),
                &QueryItem::RangeFull(RangeFull)
            );

            let (elements, _) = drive
                .grove
                .query_raw(
                    &path_query,
                    true,
                    QueryResultType::QueryPathKeyElementTrioResultType,
                    None,
                )
                .unwrap()
                .expect("expected to run the path query");
            assert_eq!(elements.len(), 7);

            let fetched_identity = drive
                .fetch_proved_full_identity(identity.id.to_buffer(), None)
                .expect("should fetch an identity")
                .expect("should have an identity");

            let (_hash, proof) = GroveDb::verify_query(fetched_identity.as_slice(), &path_query)
                .expect("expected to verify query");

            // We want to get a proof on the balance, the revision and 5 keys
            assert_eq!(proof.len(), 7);
        }
    }
}
