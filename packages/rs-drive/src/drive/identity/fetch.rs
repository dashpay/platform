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

impl Drive {
    /// Fetches the Identity's balance as an identity stub from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_stub_with_balance(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<Option<Identity>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        Ok(self
            .fetch_identity_balance_operations(
                identity_id,
                apply,
                transaction,
                &mut drive_operations,
            )?
            .map(|balance| Identity {
                protocol_version: PROTOCOL_VERSION,
                id: Identifier::new(identity_id),
                public_keys: Default::default(),
                balance,
                revision: 0,
                asset_lock_proof: None,
                metadata: None,
            }))
    }

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

    /// Fetches the Identity's revision from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_revision(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<Option<u64>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_identity_revision_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )
    }

    /// Fetches the Identity's revision from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_revision_with_fees(
        &self,
        identity_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(Option<u64>, FeeResult), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let value = self.fetch_identity_revision_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok((value, fees))
    }

    /// Creates the operations to get Identity's revision from the backing store
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn fetch_identity_revision_operations(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<u64>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetValue(1),
            }
        };
        let identity_path = identity_path(identity_id.as_slice());
        match self.grove_get_raw(
            identity_path,
            &[IdentityTreeRevision as u8],
            direct_query_type,
            transaction,
            drive_operations,
        ) {
            Ok(Some(Item(encoded_revision, _))) => {
                let revision = u64::from_be_bytes(encoded_revision.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedElementType(
                        "identity revision was not 8 bytes as expected",
                    ))
                })?);

                Ok(Some(revision))
            }

            Ok(None) => Ok(None),

            Ok(Some(..)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity revision was present but was not identified as an item",
            ))),

            Err(e) => Err(e),
        }
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub fn fetch_identity_balance_with_keys(
        &self,
        identity_key_request: IdentityKeysRequest,
        transaction: TransactionArg,
    ) -> Result<Option<Identity>, Error> {
        // let's start by getting the balance
        let id = Identifier::new(identity_key_request.identity_id);
        let balance =
            self.fetch_identity_balance(identity_key_request.identity_id, true, transaction)?;
        if balance.is_none() {
            return Ok(None);
        }
        let balance = balance.unwrap();

        let public_keys = self.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
            identity_key_request,
            transaction,
        )?;
        Ok(Some(Identity {
            protocol_version: PROTOCOL_VERSION,
            id,
            public_keys,
            balance,
            revision: u64::MAX,
            asset_lock_proof: None,
            metadata: None,
        }))
    }

    /// Fetches an identity with all its information and
    /// the cost it took from storage.
    pub fn fetch_full_identity_with_costs(
        &self,
        identity_id: [u8; 32],
        epoch: &Epoch,
        transaction: TransactionArg,
    ) -> Result<(Option<Identity>, FeeResult), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let maybe_identity =
            self.fetch_full_identity_operations(identity_id, transaction, &mut drive_operations)?;
        let fee = calculate_fee(None, Some(drive_operations), epoch)?;
        Ok((maybe_identity, fee))
    }

    /// The query getting all keys and balance and revision
    pub fn full_identity_query(identity_id: [u8; 32]) -> Result<PathQuery, Error> {
        let balance_query = Self::identity_balance_query(identity_id);
        let revision_query = Self::identity_revision_query(identity_id);
        let key_request = IdentityKeysRequest::new_all_keys_query(identity_id);
        let all_keys_query = key_request.into_path_query();
        PathQuery::merge(vec![&balance_query, &revision_query, &all_keys_query])
            .map_err(Error::GroveDB)
    }

    /// The query getting all keys and balance and revision
    pub fn full_identities_query(identity_ids: Vec<[u8; 32]>) -> Result<PathQuery, Error> {
        let path_queries: Vec<PathQuery> = identity_ids
            .into_iter()
            .map(|identity_id| Self::full_identity_query(identity_id))
            .collect::<Result<Vec<PathQuery>, Error>>()?;
        PathQuery::merge(path_queries.iter().map(|query| query).collect()).map_err(Error::GroveDB)
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_proved_full_identity(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Option<Vec<u8>>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let query = Self::full_identity_query(identity_id)?;
        let result = self.grove_get_proved_path_query(&query, transaction, &mut drive_operations);
        match result {
            Ok(r) => Ok(Some(r)),
            Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathNotFound(_))) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_proved_full_identities(
        &self,
        identity_ids: Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<Option<Vec<u8>>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let query = Self::full_identities_query(identity_ids)?;
        let result = self.grove_get_proved_path_query(&query, transaction, &mut drive_operations);
        match result {
            Ok(r) => Ok(Some(r)),
            Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathNotFound(_))) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // /// Fetches an identity with all its information from storage.
    // pub fn fetch_full_identities(
    //     &self,
    //     identity_ids: Vec<[u8; 32]>,
    //     transaction: TransactionArg,
    // ) -> Result<BTreeMap<[u8; 32],Identity>, Error> {
    //     let mut drive_operations: Vec<DriveOperation> = vec![];
    //     let query = Self::full_identities_query(identity_ids)?;
    //     let result = self.grove_get_path_query_with_optional(&query, transaction, &mut drive_operations)?;
    //     result.into_iter().map(|(_, key, element)| {
    //
    //     }).collect()
    // }

    // /// Fetches identities with all its information from storage.
    // pub fn fetch_full_identities_operations(
    //     &self,
    //     identity_ids: Vec<[u8; 32]>,
    //     transaction: TransactionArg,
    //     drive_operations: &mut Vec<DriveOperation>,
    // ) -> Result<BTreeMap<[u8; 32], Option<Identity>>, Error> {
    //     let mut drive_operations: Vec<DriveOperation> = vec![];
    //     let query = Self::full_identities_query(identity_ids)?;
    //     let result = self.grove_get_path_query_with_optional(&query, transaction, &mut drive_operations)?;
    //     result.into_iter().map(|(_, key, element)| {
    //
    //     }).collect()
    // }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_full_identity(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Option<Identity>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_full_identity_operations(identity_id, transaction, &mut drive_operations)
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub fn fetch_full_identity_operations(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<Identity>, Error> {
        // let's start by getting the balance
        let balance = self.fetch_identity_balance_operations(
            identity_id,
            true,
            transaction,
            drive_operations,
        )?;
        if balance.is_none() {
            return Ok(None);
        }
        let balance = balance.unwrap();
        let revision = self
            .fetch_identity_revision_operations(identity_id, true, transaction, drive_operations)?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "revision not found on identity".to_string(),
            )))?;

        let public_keys =
            self.fetch_all_identity_keys_operations(identity_id, transaction, drive_operations)?;
        Ok(Some(Identity {
            protocol_version: PROTOCOL_VERSION,
            id: Identifier::new(identity_id),
            public_keys,
            balance,
            revision,
            asset_lock_proof: None,
            metadata: None,
        }))
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
