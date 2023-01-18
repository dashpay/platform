use crate::drive::balances::balance_path;
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use crate::fee::result::FeeResult;
use crate::fee_pools::epochs::Epoch;
use dpp::identifier::Identifier;
use dpp::identity::Identity;
use dpp::prelude::Revision;
use grovedb::query_result_type::Path;
use grovedb::{PathQuery, TransactionArg};
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
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

    // /// Fetches identities with all its information from storage.
    // #[deprecated(since = "0.24.0", note = "please use exact fetching")]
    // pub fn fetch_full_identities_efficient(
    //     &self,
    //     identity_ids: Vec<[u8; 32]>,
    //     transaction: TransactionArg,
    // ) -> Result<BTreeMap<[u8; 32], Option<Identity>>, Error> {
    //     let mut drive_operations: Vec<DriveOperation> = vec![];
    //     let query = Self::full_identities_query(identity_ids)?;
    //     let result =
    //         self.grove_get_path_query_with_optional(&query, transaction, &mut drive_operations)?;
    //
    //     let balances_path = balance_path();
    //     // Let's do a first pass to get identities from balances
    //     let mut identities :BTreeMap<[u8; 32], Option<Identity>> = result.iter().filter_map(
    //         |(path, key, element)| {
    //             if path == balances_path {
    //                 let identity_id = key.try_into().map_err(|_| Error::Drive(DriveError::CorruptedDriveState("balance key not 32 bytes".to_string())))?;
    //                 if &Some(balance) = element {
    //                     Some((identity_id, Identity {
    //                         protocol_version: PROTOCOL_VERSION,
    //                         id: identity_id,
    //                         public_keys: Default::default(),
    //                         balance,
    //                         revision: Revision::MAX,
    //                         asset_lock_proof: None,
    //                         metadata: None,
    //                     }))
    //                 } else {
    //                     Some((identity_id, None))
    //                 }
    //             } else {
    //                 None
    //             }
    //         }
    //     ).collect()?;
    //
    //     result.into_iter().try_for_each(
    //         |(path, key, element)| {
    //             if path != balances_path {
    //                 if let Some(element) = element {
    //                     // we need to get the identity_id from the path which will be the second item (1)
    //                     // of the path
    //                     let identity_id: [u8;32] = path.get(1).ok_or(Error::Drive(DriveError::CorruptedDriveState("path much contain identity id".to_string())))?.try_into()
    //                         .try_into().map_err(|_| Error::Drive(DriveError::CorruptedDriveState("identity id not 32 bytes".to_string())))?;
    //                     let mut identity = identities.get_mut(&identity_id).ok_or();
    //                 }
    //             }
    //         }
    //     )?;
    //     Ok(identities)
    // }

    /// Fetches identities with all its information from storage.
    #[deprecated(since = "0.24.0", note = "please use exact fetching")]
    pub fn fetch_full_identities(
        &self,
        identity_ids: Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<BTreeMap<[u8; 32], Option<Identity>>, Error> {
        identity_ids
            .into_iter()
            .map(|identity_id| {
                Ok((
                    identity_id,
                    self.fetch_full_identity(identity_id, transaction)?,
                ))
            })
            .collect()
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    use grovedb::GroveDb;

    mod fetch_full_identities {
        use super::*;
        use crate::drive::block_info::BlockInfo;

        #[test]
        fn should_get_full_identities() {
            let drive = setup_drive_with_initial_state_structure();

            let identities: BTreeMap<[u8; 32], Option<Identity>> =
                Identity::random_identities(10, 3, Some(14))
                    .into_iter()
                    .map(|identity| (identity.id.to_buffer(), Some(identity)))
                    .collect();

            for identity in identities.values() {
                drive
                    .add_new_identity(
                        identity.as_ref().unwrap().clone(),
                        &BlockInfo::default(),
                        true,
                        None,
                    )
                    .expect("expected to add an identity");
            }
            let fetched_identities = drive
                .fetch_full_identities(identities.keys().copied().collect(), None)
                .expect("should get identities");

            assert_eq!(identities, fetched_identities);
        }
    }

    mod fetch_full_identity {
        use super::*;
        use crate::drive::block_info::BlockInfo;

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

        #[test]
        fn should_get_a_full_identity() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(3, Some(14));

            let identity_id = identity.id.to_buffer();
            drive
                .add_new_identity(identity.clone(), &BlockInfo::default(), true, None)
                .expect("expected to add an identity");
            let fetched_identity = drive
                .fetch_full_identity(identity_id, None)
                .expect("should not error when fetching an identity")
                .expect("should find an identity");

            assert_eq!(identity, fetched_identity);
        }
    }

    mod fetch_proved_full_identities {
        use super::*;
        use crate::drive::block_info::BlockInfo;
        use grovedb::query_result_type::QueryResultType;
        use grovedb::QueryItem;
        use std::ops::RangeFull;

        #[test]
        fn test_proved_full_identities_query_no_tx() {
            let drive = setup_drive_with_initial_state_structure();

            let identities: BTreeMap<[u8; 32], Option<Identity>> =
                Identity::random_identities(2, 5, Some(14))
                    .into_iter()
                    .map(|identity| (identity.id.to_buffer(), Some(identity)))
                    .collect();

            for identity in identities.values() {
                drive
                    .add_new_identity(
                        identity.as_ref().unwrap().clone(),
                        &BlockInfo::default(),
                        true,
                        None,
                    )
                    .expect("expected to add an identity");
            }

            let path_query = Drive::full_identities_query(identities.keys().copied().collect())
                .expect("expected to get query");

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
            assert_eq!(elements.len(), 70);

            let fetched_identities = drive
                .fetch_proved_full_identities(identities.keys().copied().collect(), None)
                .expect("should fetch an identity")
                .expect("should have an identity");

            let (_hash, proof) = GroveDb::verify_query(fetched_identities.as_slice(), &path_query)
                .expect("expected to verify query");

            // We want to get a proof on the balance, the revision and 5 keys
            assert_eq!(proof.len(), 70);
        }
    }

    mod fetch_proved_full_identity {
        use super::*;
        use crate::drive::block_info::BlockInfo;
        use grovedb::query_result_type::QueryResultType;
        use grovedb::QueryItem;
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
