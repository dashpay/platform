mod fetch_full_identities;
mod fetch_full_identity;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    mod fetch_full_identities {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::version::PlatformVersion;
        use std::collections::BTreeMap;

        #[test]
        fn should_get_full_identities() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identities: BTreeMap<[u8; 32], Option<Identity>> =
                Identity::random_identities(10, 3, Some(14), platform_version)
                    .expect("expected to get random identities")
                    .into_iter()
                    .map(|identity| (identity.id().to_buffer(), Some(identity)))
                    .collect();

            for identity in identities.values() {
                drive
                    .add_new_identity(
                        identity.as_ref().unwrap().clone(),
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to add an identity");
            }
            let fetched_identities = drive
                .fetch_full_identities(
                    identities.keys().copied().collect::<Vec<_>>().as_slice(),
                    None,
                    platform_version,
                )
                .expect("should get identities");

            assert_eq!(identities, fetched_identities);
        }
    }

    mod fetch_full_identities_additional {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_return_none_for_non_existent_ids_in_batch() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(14), platform_version)
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
                .expect("expected to add an identity");

            let non_existent_id = [0xffu8; 32];
            let ids = vec![identity.id().to_buffer(), non_existent_id];
            let fetched = drive
                .fetch_full_identities(&ids, None, platform_version)
                .expect("should get identities");

            assert_eq!(fetched.len(), 2);
            assert!(fetched[&identity.id().to_buffer()].is_some());
            assert!(fetched[&non_existent_id].is_none());
        }

        #[test]
        fn should_return_empty_map_for_empty_input() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let fetched = drive
                .fetch_full_identities(&[], None, platform_version)
                .expect("should get empty result");

            assert!(fetched.is_empty());
        }
    }

    mod fetch_full_identity {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_return_none_if_identity_is_not_present() {
            let drive = setup_drive_with_initial_state_structure(None);

            let platform_version = PlatformVersion::latest();

            let identity = drive
                .fetch_full_identity(
                    [
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0,
                    ],
                    None,
                    platform_version,
                )
                .expect("should return none");

            assert!(identity.is_none());
        }

        #[test]
        fn should_get_a_full_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(14), platform_version)
                .expect("expected a random identity");

            let identity_id = identity.id().to_buffer();
            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add an identity");
            let fetched_identity = drive
                .fetch_full_identity(identity_id, None, platform_version)
                .expect("should not error when fetching an identity")
                .expect("should find an identity");

            assert_eq!(identity, fetched_identity);
        }
    }

    mod fetch_full_identity_with_costs {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::block::epoch::Epoch;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_return_none_with_fee_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let epoch = Epoch::new(0).expect("expected epoch");

            let (identity, fee) = drive
                .fetch_full_identity_with_costs([0u8; 32], &epoch, None, platform_version)
                .expect("should return none with fee");

            assert!(identity.is_none());
            assert!(fee.processing_fee > 0);
        }

        #[test]
        fn should_return_identity_with_fee_for_existing_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let epoch = Epoch::new(0).expect("expected epoch");

            let identity = Identity::random_identity(3, Some(14), platform_version)
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
                .expect("expected to add an identity");

            let (fetched_identity, fee) = drive
                .fetch_full_identity_with_costs(
                    identity.id().to_buffer(),
                    &epoch,
                    None,
                    platform_version,
                )
                .expect("should return identity with fee");

            assert_eq!(fetched_identity.unwrap(), identity);
            assert!(fee.processing_fee > 0);
        }
    }

    mod fetch_full_identity_operations {
        use super::*;
        use crate::fees::op::LowLevelDriveOperation;
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_return_none_for_non_existent_identity_operations() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

            let identity = drive
                .fetch_full_identity_operations(
                    [0u8; 32],
                    None,
                    &mut drive_operations,
                    platform_version,
                )
                .expect("should return none");

            assert!(identity.is_none());
            assert!(!drive_operations.is_empty());
        }

        #[test]
        fn should_return_identity_and_record_operations() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(14), platform_version)
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
                .expect("expected to add an identity");

            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
            let fetched_identity = drive
                .fetch_full_identity_operations(
                    identity.id().to_buffer(),
                    None,
                    &mut drive_operations,
                    platform_version,
                )
                .expect("should return identity")
                .expect("should have identity");

            assert_eq!(fetched_identity, identity);
            assert!(!drive_operations.is_empty());
        }
    }

    mod fetch_full_identity_with_transaction {
        use crate::config::DriveConfig;
        use crate::util::test_helpers::setup::setup_drive;
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_fetch_identity_within_transaction() {
            let drive = setup_drive(Some(DriveConfig {
                batching_consistency_verification: true,
                ..Default::default()
            }));
            let platform_version = PlatformVersion::latest();

            let transaction = drive.grove.start_transaction();
            drive
                .create_initial_state_structure(Some(&transaction), platform_version)
                .expect("should create root tree");

            let identity = Identity::random_identity(3, Some(42), platform_version)
                .expect("expected a random identity");

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to add identity");

            let fetched = drive
                .fetch_full_identity(
                    identity.id().to_buffer(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("should not error")
                .expect("should find identity in transaction");

            assert_eq!(fetched, identity);

            let fetched_outside = drive
                .fetch_full_identity(identity.id().to_buffer(), None, platform_version)
                .expect("should not error");

            assert!(fetched_outside.is_none());
        }
    }
}
