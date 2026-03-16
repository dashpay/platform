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
            .map_err(Error::from)?;

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
            .map_err(Error::from)?;

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
                limit: Some(self.config.max_query_limit),
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
            .map_err(Error::from)?;

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
            .map_err(Error::from)?;

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

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    mod verify_all_identities_exist {
        use super::*;

        #[test]
        fn should_return_true_when_all_identities_exist() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identities: Vec<Identity> =
                Identity::random_identities(3, 3, Some(42), platform_version)
                    .expect("expected random identities");

            let ids: Vec<[u8; 32]> = identities.iter().map(|i| i.id().to_buffer()).collect();

            for identity in &identities {
                drive
                    .add_new_identity(
                        identity.clone(),
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to add identity");
            }

            let result = drive
                .verify_all_identities_exist(&ids, None, platform_version)
                .expect("should not error");

            assert!(result);
        }

        #[test]
        fn should_return_false_when_some_identities_missing() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(42), platform_version)
                .expect("expected a random identity");

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add identity");

            // Include a nonexistent identity id
            let ids = vec![identity.id().to_buffer(), [0xff; 32]];

            let result = drive
                .verify_all_identities_exist(&ids, None, platform_version)
                .expect("should not error");

            assert!(!result);
        }
    }

    mod fetch_identities_balances {
        use super::*;

        #[test]
        fn should_fetch_balances_for_existing_identities() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identities: Vec<Identity> =
                Identity::random_identities(3, 3, Some(42), platform_version)
                    .expect("expected random identities");

            for identity in &identities {
                drive
                    .add_new_identity(
                        identity.clone(),
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to add identity");
            }

            let ids: Vec<[u8; 32]> = identities.iter().map(|i| i.id().to_buffer()).collect();

            let balances = drive
                .fetch_identities_balances(&ids, None, platform_version)
                .expect("should fetch balances");

            assert_eq!(balances.len(), 3);
            for identity in &identities {
                let balance = balances
                    .get(&identity.id().to_buffer())
                    .expect("should have balance for identity");
                assert_eq!(*balance, identity.balance());
            }
        }

        #[test]
        fn should_return_only_existing_identities_when_some_are_missing() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identities: Vec<Identity> =
                Identity::random_identities(2, 3, Some(42), platform_version)
                    .expect("expected random identities");

            // Only insert the first identity
            drive
                .add_new_identity(
                    identities[0].clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add identity");

            // Query for both the existing identity and a non-existent one
            let ids = vec![
                identities[0].id().to_buffer(),
                identities[1].id().to_buffer(),
            ];

            let balances = drive
                .fetch_identities_balances(&ids, None, platform_version)
                .expect("should fetch balances");

            // Only the inserted identity should be returned
            assert_eq!(balances.len(), 1);
            let balance = balances
                .get(&identities[0].id().to_buffer())
                .expect("should have balance for existing identity");
            assert_eq!(*balance, identities[0].balance());
            assert!(
                balances.get(&identities[1].id().to_buffer()).is_none(),
                "non-existent identity should not appear in results"
            );
        }
    }

    mod fetch_optional_identities_balances {
        use super::*;

        #[test]
        fn should_return_none_for_non_existent_identities() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(42), platform_version)
                .expect("expected a random identity");

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add identity");

            let ids = vec![identity.id().to_buffer(), [0xff; 32]];

            let balances = drive
                .fetch_optional_identities_balances(&ids, None, platform_version)
                .expect("should fetch optional balances");

            assert_eq!(balances.len(), 2);
            assert_eq!(
                *balances
                    .get(&identity.id().to_buffer())
                    .expect("existing identity should have an entry in results"),
                Some(identity.balance())
            );
            assert_eq!(
                *balances
                    .get(&[0xff; 32])
                    .expect("non-existent identity should still have an entry in results"),
                None
            );
        }
    }

    mod fetch_many_identity_balances_by_range {
        use super::*;
        use std::collections::BTreeMap;

        #[test]
        fn should_fetch_balances_by_range_ascending() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identities: Vec<Identity> =
                Identity::random_identities(5, 3, Some(42), platform_version)
                    .expect("expected random identities");

            for identity in &identities {
                drive
                    .add_new_identity(
                        identity.clone(),
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to add identity");
            }

            let balances: BTreeMap<[u8; 32], u64> = drive
                .fetch_many_identity_balances_by_range::<BTreeMap<[u8; 32], u64>>(
                    None,
                    true,
                    10,
                    None,
                    platform_version,
                )
                .expect("should fetch balances by range");

            assert_eq!(balances.len(), 5);

            // Verify that every inserted identity appears with its correct balance
            for identity in &identities {
                let balance = balances
                    .get(&identity.id().to_buffer())
                    .expect("should contain balance for inserted identity");
                assert_eq!(*balance, identity.balance());
            }
        }

        #[test]
        fn should_respect_limit() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identities: Vec<Identity> =
                Identity::random_identities(5, 3, Some(42), platform_version)
                    .expect("expected random identities");

            for identity in &identities {
                drive
                    .add_new_identity(
                        identity.clone(),
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to add identity");
            }

            let balances: BTreeMap<[u8; 32], u64> = drive
                .fetch_many_identity_balances_by_range::<BTreeMap<[u8; 32], u64>>(
                    None,
                    true,
                    2,
                    None,
                    platform_version,
                )
                .expect("should fetch balances by range");

            assert_eq!(balances.len(), 2);
        }
    }
}
