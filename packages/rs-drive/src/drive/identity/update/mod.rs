pub(crate) mod methods;
mod operations;
mod structs;

pub use structs::*;

#[cfg(test)]
mod tests {

    use dpp::prelude::*;

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::identity::accessors::IdentityGettersV0;

    mod add_new_keys_to_identity {
        use super::*;
        use crate::drive::Drive;
        use crate::fees::op::LowLevelDriveOperation;
        use dpp::block::block_info::BlockInfo;
        use dpp::block::epoch::Epoch;
        use dpp::fee::fee_result::FeeResult;
        use dpp::version::PlatformVersion;
        use rand::prelude::StdRng;
        use rand::{Rng, SeedableRng};

        #[test]
        fn should_add_one_new_key_to_identity_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 14202000,
                processing_fee: 1098260,
                ..Default::default()
            };

            should_add_one_new_key_to_identity(platform_version, expected_fee_result);
        }

        #[test]
        fn should_add_one_new_key_to_identity_latest_version() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 14202000,
                processing_fee: 1098260,
                ..Default::default()
            };

            should_add_one_new_key_to_identity(platform_version, expected_fee_result);
        }

        fn should_add_one_new_key_to_identity(
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(None);

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            let new_keys_to_add =
                IdentityPublicKey::random_authentication_keys(5, 1, Some(15), platform_version);

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .add_new_unique_keys_to_identity(
                    identity.id().to_buffer(),
                    new_keys_to_add,
                    &block,
                    true,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("expected to update identity with new keys");

            assert_eq!(fee_result, expected_fee_result);

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let identity_keys = drive
                .fetch_all_identity_keys(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get balance");

            assert_eq!(identity_keys.len(), 6); // we had 5 keys and we added 1
        }

        #[test]
        fn check_reference_below_tokens_cost_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                processing_fee: 406100,
                ..Default::default()
            };
            check_reference_below_tokens_cost(platform_version, expected_fee_result);
        }

        #[test]
        fn check_reference_below_tokens_cost_latest_version() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                processing_fee: 406100,
                ..Default::default()
            };
            check_reference_below_tokens_cost(platform_version, expected_fee_result);
        }

        fn check_reference_below_tokens_cost(
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(None);

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            IdentityPublicKey::random_authentication_keys(5, 1, Some(15), platform_version);

            let mut rng = StdRng::seed_from_u64(23450);

            let db_transaction = drive.grove.start_transaction();

            let batch_operations = drive
                .insert_non_unique_public_key_hash_reference_to_identity_operations(
                    identity.id().to_buffer(),
                    rng.gen(),
                    &mut None,
                    Some(&db_transaction),
                    &platform_version.drive,
                )
                .expect("expected to update identity with new keys");

            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&db_transaction),
                    batch_operations,
                    &mut drive_operations,
                    &platform_version.drive,
                )
                .expect("expected to apply operations");

            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &Epoch::new(0).unwrap(),
                drive.config.epochs_per_era,
                platform_version,
                None,
            )
            .expect("expected fee result");

            assert_eq!(fee_result, expected_fee_result);

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");
        }

        #[test]
        fn should_add_two_dozen_new_keys_to_identity_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 347382000,
                processing_fee: 6819220,
                ..Default::default()
            };

            should_add_two_dozen_new_keys_to_identity(platform_version, expected_fee_result);
        }

        #[test]
        fn should_add_two_dozen_new_keys_to_identity_latest_version() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 347382000,
                processing_fee: 6819220,
                ..Default::default()
            };

            should_add_two_dozen_new_keys_to_identity(platform_version, expected_fee_result);
        }

        fn should_add_two_dozen_new_keys_to_identity(
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(None);

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            let new_keys_to_add =
                IdentityPublicKey::random_authentication_keys(5, 24, Some(15), platform_version);

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .add_new_unique_keys_to_identity(
                    identity.id().to_buffer(),
                    new_keys_to_add,
                    &block,
                    true,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("expected to update identity with new keys");

            assert_eq!(fee_result, expected_fee_result);

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let identity_keys = drive
                .fetch_all_identity_keys(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get balance");

            assert_eq!(identity_keys.len(), 29); // we had 5 keys and we added 24
        }

        #[test]
        fn should_estimated_costs_without_state_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 17145000,
                processing_fee: 5483620,
                ..Default::default()
            };

            should_estimated_costs_without_state_add_new_keys(
                platform_version,
                expected_fee_result,
            );
        }

        #[test]
        fn should_estimated_costs_without_state_latest_version() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 17145000,
                processing_fee: 5483620,
                ..Default::default()
            };

            should_estimated_costs_without_state_add_new_keys(
                platform_version,
                expected_fee_result,
            );
        }

        fn should_estimated_costs_without_state_add_new_keys(
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(None);

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let new_keys_to_add =
                IdentityPublicKey::random_authentication_keys(5, 1, Some(15), platform_version);

            let app_hash_before = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            let fee_result = drive
                .add_new_unique_keys_to_identity(
                    identity.id().to_buffer(),
                    new_keys_to_add,
                    &block,
                    false,
                    None,
                    platform_version,
                )
                .expect("expected to update identity with new keys");

            let app_hash_after = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);
            assert_eq!(fee_result, expected_fee_result);
        }
    }

    mod disable_identity_keys {
        use super::*;
        use chrono::Utc;
        use dpp::block::block_info::BlockInfo;
        use dpp::block::epoch::Epoch;
        use dpp::fee::fee_result::FeeResult;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_disable_a_few_keys_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 513000,
                processing_fee: 869380,
                ..Default::default()
            };
            should_disable_a_few_keys(platform_version, expected_fee_result);
        }
        #[test]
        fn should_disable_a_few_keys_latest_version() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 513000,
                processing_fee: 869380,
                ..Default::default()
            };
            should_disable_a_few_keys(platform_version, expected_fee_result);
        }
        fn should_disable_a_few_keys(
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(None);

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            let new_keys_to_add = IdentityPublicKey::random_keys(5, 2, Some(15), platform_version);

            drive
                .add_new_unique_keys_to_identity(
                    identity.id().to_buffer(),
                    new_keys_to_add.clone(),
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to update identity with new keys");

            let db_transaction = drive.grove.start_transaction();

            let key_ids = new_keys_to_add.into_iter().map(|key| key.id()).collect();

            let disable_at = Utc::now().timestamp_millis() as TimestampMillis;

            let fee_result = drive
                .disable_identity_keys(
                    identity.id().to_buffer(),
                    key_ids,
                    disable_at,
                    &block_info,
                    true,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("should disable a few keys");

            assert_eq!(fee_result, expected_fee_result,);

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let identity_keys = drive
                .fetch_all_identity_keys(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get balance");

            assert_eq!(identity_keys.len(), 7); // we had 5 keys and we added 2

            for (_, key) in identity_keys.into_iter().skip(5) {
                assert_eq!(key.disabled_at(), Some(disable_at));
            }
        }

        #[test]
        fn should_estimated_costs_without_state_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };

            should_estimated_costs_without_state_disable_keys(
                platform_version,
                expected_fee_result,
            );
        }

        #[test]
        fn should_estimated_costs_without_state_latest_version() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };

            should_estimated_costs_without_state_disable_keys(
                platform_version,
                expected_fee_result,
            );
        }

        fn should_estimated_costs_without_state_disable_keys(
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(None);

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let disable_at = Utc::now().timestamp_millis() as TimestampMillis;

            let app_hash_before = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            let fee_result = drive
                .disable_identity_keys(
                    identity.id().to_buffer(),
                    vec![0, 1],
                    disable_at,
                    &block_info,
                    false,
                    None,
                    platform_version,
                )
                .expect("should estimate the disabling of a few keys");

            let app_hash_after = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);
            assert_eq!(fee_result, expected_fee_result);
        }

        #[test]
        fn estimated_costs_should_have_same_storage_cost_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_estimated_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 794720,
                ..Default::default()
            };
            estimated_costs_should_have_same_storage_cost(
                platform_version,
                expected_estimated_fee_result,
                expected_fee_result,
            );
        }

        #[test]
        fn estimated_costs_should_have_same_storage_cost_latest_version() {
            let platform_version = PlatformVersion::latest();
            let expected_estimated_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 794720,
                ..Default::default()
            };
            estimated_costs_should_have_same_storage_cost(
                platform_version,
                expected_estimated_fee_result,
                expected_fee_result,
            );
        }

        fn estimated_costs_should_have_same_storage_cost(
            platform_version: &PlatformVersion,
            expected_estimated_fee_result: FeeResult,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(None);

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
                .expect("expected to add an identity");

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let disable_at = Utc::now().timestamp_millis() as TimestampMillis;

            let estimated_fee_result = drive
                .disable_identity_keys(
                    identity.id().to_buffer(),
                    vec![0, 1],
                    disable_at,
                    &block_info,
                    false,
                    None,
                    platform_version,
                )
                .expect("should estimate the disabling of a few keys");

            let fee_result = drive
                .disable_identity_keys(
                    identity.id().to_buffer(),
                    vec![0, 1],
                    disable_at,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("should get the cost of the disabling a few keys");

            assert_eq!(estimated_fee_result.storage_fee, fee_result.storage_fee);
            assert_eq!(estimated_fee_result, expected_estimated_fee_result);
            assert_eq!(fee_result, expected_fee_result);
        }
    }

    mod update_identity_revision {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::block::epoch::Epoch;
        use dpp::fee::fee_result::FeeResult;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_update_revision_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 238820,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            should_update_revision(platform_version, expected_fee_result);
        }

        #[test]
        fn should_update_revision_latest_version() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 238820,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            should_update_revision(platform_version, expected_fee_result);
        }

        fn should_update_revision(
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(None);

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");
            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            let revision = 2;

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .update_identity_revision(
                    identity.id().to_buffer(),
                    revision,
                    &block_info,
                    true,
                    Some(&db_transaction),
                    platform_version,
                    None,
                )
                .expect("should update revision");

            assert_eq!(fee_result, expected_fee_result);

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let updated_revision = drive
                .fetch_identity_revision(identity.id().to_buffer(), true, None, platform_version)
                .expect("expected to get revision");

            assert_eq!(updated_revision, Some(revision));
        }

        #[test]
        fn should_estimated_costs_without_state_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 1813560,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            should_estimated_costs_without_state_update_identity_revision(
                platform_version,
                expected_fee_result,
            );
        }

        #[test]
        fn should_estimated_costs_without_state_latest_version() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 1813560,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            should_estimated_costs_without_state_update_identity_revision(
                platform_version,
                expected_fee_result,
            );
        }

        fn should_estimated_costs_without_state_update_identity_revision(
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(None);

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let revision = 2;

            let app_hash_before = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            let fee_result = drive
                .update_identity_revision(
                    identity.id().to_buffer(),
                    revision,
                    &block_info,
                    false,
                    None,
                    platform_version,
                    None,
                )
                .expect("should estimate the revision update");

            let app_hash_after = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);
            assert_eq!(fee_result, expected_fee_result);
        }
    }
}
