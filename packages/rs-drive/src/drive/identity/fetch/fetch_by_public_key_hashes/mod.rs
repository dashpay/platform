mod fetch_full_identities_by_unique_public_key_hashes;
mod fetch_full_identity_by_non_unique_public_key_hash;
mod fetch_full_identity_by_unique_public_key_hash;
mod fetch_identity_id_by_unique_public_key_hash;
mod fetch_identity_ids_by_non_unique_public_key_hash;
mod fetch_identity_ids_by_unique_public_key_hashes;
mod has_any_of_unique_public_key_hashes;
mod has_non_unique_public_key_hash;
mod has_non_unique_public_key_hash_already_for_identity;
mod has_unique_public_key_hash;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_fetch_all_keys_on_identity() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345), platform_version)
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
            .expect("expected to insert identity");

        let public_keys = drive
            .fetch_all_identity_keys(
                identity.id().to_buffer(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 5);

        for (_, key) in public_keys {
            let hash = key.public_key_hash().expect("expected to get hash");
            if key.key_type().is_unique_key_type() {
                let identity_id = drive
                    .fetch_identity_id_by_unique_public_key_hash(
                        hash,
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("expected to fetch identity_id")
                    .expect("expected to get an identity id");
                assert_eq!(identity_id, identity.id().to_buffer());
            } else {
                let identity_ids = drive
                    .fetch_identity_ids_by_non_unique_public_key_hash(
                        hash,
                        None,
                        None,
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("expected to get identity ids");
                assert!(identity_ids.contains(&identity.id().to_buffer()));
            }
        }
    }

    mod fetch_identity_id_by_unique_public_key_hash {
        use super::*;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_none_for_unknown_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let unknown_hash = [0xabu8; 20];
            let result = drive
                .fetch_identity_id_by_unique_public_key_hash(unknown_hash, None, platform_version)
                .expect("should not error");

            assert!(result.is_none());
        }

        #[test]
        fn should_return_identity_id_for_known_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(777), platform_version)
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

            let unique_key = identity
                .public_keys()
                .values()
                .find(|k| k.key_type().is_unique_key_type())
                .expect("should have a unique key");

            let hash = unique_key.public_key_hash().expect("should hash");

            let fetched_id = drive
                .fetch_identity_id_by_unique_public_key_hash(hash, None, platform_version)
                .expect("should not error")
                .expect("should find identity id");

            assert_eq!(fetched_id, identity.id().to_buffer());
        }
    }

    mod fetch_full_identity_by_unique_public_key_hash {
        use super::*;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_none_for_unknown_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let unknown_hash = [0xcdu8; 20];
            let result = drive
                .fetch_full_identity_by_unique_public_key_hash(unknown_hash, None, platform_version)
                .expect("should not error");

            assert!(result.is_none());
        }

        #[test]
        fn should_return_full_identity_for_known_unique_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(888), platform_version)
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

            let unique_key = identity
                .public_keys()
                .values()
                .find(|k| k.key_type().is_unique_key_type())
                .expect("should have a unique key");

            let hash = unique_key.public_key_hash().expect("should hash");

            let fetched = drive
                .fetch_full_identity_by_unique_public_key_hash(hash, None, platform_version)
                .expect("should not error")
                .expect("should find identity");

            assert_eq!(fetched, identity);
        }
    }

    mod has_unique_public_key_hash {
        use super::*;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_false_for_unknown_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let unknown_hash = [0xefu8; 20];
            let result = drive
                .has_unique_public_key_hash(unknown_hash, None, &platform_version.drive)
                .expect("should not error");

            assert!(!result);
        }

        #[test]
        fn should_return_true_for_known_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(999), platform_version)
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

            let unique_key = identity
                .public_keys()
                .values()
                .find(|k| k.key_type().is_unique_key_type())
                .expect("should have a unique key");

            let hash = unique_key.public_key_hash().expect("should hash");

            let result = drive
                .has_unique_public_key_hash(hash, None, &platform_version.drive)
                .expect("should not error");

            assert!(result);
        }
    }

    mod has_non_unique_public_key_hash {
        use super::*;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_false_for_unknown_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let unknown_hash = [0x11u8; 20];
            let result = drive
                .has_non_unique_public_key_hash(unknown_hash, None, &platform_version.drive)
                .expect("should not error");

            assert!(!result);
        }

        #[test]
        fn should_return_true_for_identity_with_non_unique_key() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(12345), platform_version)
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

            let non_unique_key = identity
                .public_keys()
                .values()
                .find(|k| !k.key_type().is_unique_key_type())
                .expect("random identity should have at least one non-unique key");

            let hash = non_unique_key.public_key_hash().expect("should hash");
            let result = drive
                .has_non_unique_public_key_hash(hash, None, &platform_version.drive)
                .expect("should not error");
            assert!(result, "expected non-unique key hash to be found");
        }
    }

    mod has_any_of_unique_public_key_hashes {
        use super::*;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_empty_for_unknown_hashes() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let hashes = vec![[0x22u8; 20], [0x33u8; 20]];
            let result = drive
                .has_any_of_unique_public_key_hashes(hashes, None, platform_version)
                .expect("should not error");

            assert!(result.is_empty());
        }

        #[test]
        fn should_return_matching_hashes_for_known_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(555), platform_version)
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

            let mut hashes: Vec<[u8; 20]> = identity
                .public_keys()
                .values()
                .filter(|k| k.key_type().is_unique_key_type())
                .map(|k| k.public_key_hash().expect("should hash"))
                .collect();

            hashes.push([0xffu8; 20]);

            let result = drive
                .has_any_of_unique_public_key_hashes(hashes.clone(), None, platform_version)
                .expect("should not error");

            assert!(!result.is_empty());
            assert!(!result.contains(&[0xffu8; 20]));
        }
    }

    mod fetch_identity_ids_by_unique_public_key_hashes {
        use super::*;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_none_for_unknown_hashes() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let hashes = [[0x44u8; 20], [0x55u8; 20]];
            let result = drive
                .fetch_identity_ids_by_unique_public_key_hashes(&hashes, None, platform_version)
                .expect("should not error");

            assert_eq!(result.len(), 2);
            for id in result.values() {
                assert!(id.is_none());
            }
        }

        #[test]
        fn should_return_identity_ids_for_known_hashes() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(666), platform_version)
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

            let unique_hashes: Vec<[u8; 20]> = identity
                .public_keys()
                .values()
                .filter(|k| k.key_type().is_unique_key_type())
                .map(|k| k.public_key_hash().expect("should hash"))
                .collect();

            let result = drive
                .fetch_identity_ids_by_unique_public_key_hashes(
                    &unique_hashes,
                    None,
                    platform_version,
                )
                .expect("should not error");

            for hash in &unique_hashes {
                let id = result
                    .get(hash)
                    .expect("hash should be in results")
                    .expect("identity id should be Some");
                assert_eq!(id, identity.id().to_buffer());
            }
        }

        #[test]
        fn should_handle_mix_of_known_and_unknown_hashes() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(667), platform_version)
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

            let known_hash = identity
                .public_keys()
                .values()
                .find(|k| k.key_type().is_unique_key_type())
                .expect("should have unique key")
                .public_key_hash()
                .expect("should hash");

            let unknown_hash = [0x77u8; 20];
            let hashes = vec![known_hash, unknown_hash];

            let result = drive
                .fetch_identity_ids_by_unique_public_key_hashes(&hashes, None, platform_version)
                .expect("should not error");

            assert_eq!(result.len(), 2);
            assert!(result[&known_hash].is_some());
            assert!(result[&unknown_hash].is_none());
        }
    }

    mod fetch_full_identities_by_unique_public_key_hashes {
        use super::*;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_none_for_unknown_hashes() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let hashes = [[0x88u8; 20]];
            let result = drive
                .fetch_full_identities_by_unique_public_key_hashes(&hashes, None, platform_version)
                .expect("should not error");

            assert_eq!(result.len(), 1);
            assert!(result[&[0x88u8; 20]].is_none());
        }

        #[test]
        fn should_return_identities_for_known_hashes() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(1111), platform_version)
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

            let unique_hashes: Vec<[u8; 20]> = identity
                .public_keys()
                .values()
                .filter(|k| k.key_type().is_unique_key_type())
                .map(|k| k.public_key_hash().expect("should hash"))
                .collect();

            let result = drive
                .fetch_full_identities_by_unique_public_key_hashes(
                    &unique_hashes,
                    None,
                    platform_version,
                )
                .expect("should not error");

            for hash in &unique_hashes {
                let fetched = result
                    .get(hash)
                    .expect("hash should be in results")
                    .as_ref()
                    .expect("identity should be Some");
                assert_eq!(*fetched, identity);
            }
        }
    }

    mod fetch_full_identity_by_non_unique_public_key_hash {
        use super::*;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_none_for_unknown_non_unique_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let unknown_hash = [0x99u8; 20];
            let result = drive
                .fetch_full_identity_by_non_unique_public_key_hash(
                    unknown_hash,
                    None,
                    None,
                    platform_version,
                )
                .expect("should not error");

            assert!(result.is_none());
        }

        #[test]
        fn should_return_identity_for_known_non_unique_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(2222), platform_version)
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

            let non_unique_key = identity
                .public_keys()
                .values()
                .find(|k| !k.key_type().is_unique_key_type());

            if let Some(key) = non_unique_key {
                let hash = key.public_key_hash().expect("should hash");
                let result = drive
                    .fetch_full_identity_by_non_unique_public_key_hash(
                        hash,
                        None,
                        None,
                        platform_version,
                    )
                    .expect("should not error");

                assert!(result.is_some());
                assert_eq!(result.unwrap(), identity);
            }
        }
    }

    mod fetch_identity_ids_by_non_unique_public_key_hash {
        use super::*;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_empty_for_unknown_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let unknown_hash = [0xaau8; 20];
            let result = drive
                .fetch_identity_ids_by_non_unique_public_key_hash(
                    unknown_hash,
                    None,
                    None,
                    None,
                    platform_version,
                )
                .expect("should not error");

            assert!(result.is_empty());
        }

        #[test]
        fn should_return_identity_id_for_known_hash() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(3333), platform_version)
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

            let non_unique_key = identity
                .public_keys()
                .values()
                .find(|k| !k.key_type().is_unique_key_type());

            if let Some(key) = non_unique_key {
                let hash = key.public_key_hash().expect("should hash");
                let result = drive
                    .fetch_identity_ids_by_non_unique_public_key_hash(
                        hash,
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .expect("should not error");

                assert!(!result.is_empty());
                assert!(result.contains(&identity.id().to_buffer()));
            }
        }

        #[test]
        fn should_respect_limit() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(4444), platform_version)
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

            let non_unique_key = identity
                .public_keys()
                .values()
                .find(|k| !k.key_type().is_unique_key_type());

            if let Some(key) = non_unique_key {
                let hash = key.public_key_hash().expect("should hash");
                let result = drive
                    .fetch_identity_ids_by_non_unique_public_key_hash(
                        hash,
                        Some(1),
                        None,
                        None,
                        platform_version,
                    )
                    .expect("should not error");

                assert!(result.len() <= 1);
            }
        }
    }

    mod has_non_unique_public_key_hash_already_for_identity {
        use super::*;
        use crate::fees::op::LowLevelDriveOperation;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn should_return_false_for_wrong_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(5555), platform_version)
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

            let non_unique_key = identity
                .public_keys()
                .values()
                .find(|k| !k.key_type().is_unique_key_type());

            if let Some(key) = non_unique_key {
                let hash = key.public_key_hash().expect("should hash");
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let result = drive
                    .has_non_unique_public_key_hash_already_for_identity_operations(
                        hash,
                        [0xffu8; 32],
                        None,
                        &mut drive_operations,
                        &platform_version.drive,
                    )
                    .expect("should not error");

                assert!(!result);
            }
        }

        #[test]
        fn should_return_true_for_correct_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(6666), platform_version)
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

            let non_unique_key = identity
                .public_keys()
                .values()
                .find(|k| !k.key_type().is_unique_key_type());

            if let Some(key) = non_unique_key {
                let hash = key.public_key_hash().expect("should hash");
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let result = drive
                    .has_non_unique_public_key_hash_already_for_identity_operations(
                        hash,
                        identity.id().to_buffer(),
                        None,
                        &mut drive_operations,
                        &platform_version.drive,
                    )
                    .expect("should not error");

                assert!(result);
            }
        }
    }
}
