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

        // -------------------------------------------------
        // should_add_one_new_key_to_identity (4 tests)
        // -------------------------------------------------
        #[test]
        fn should_add_one_new_key_to_identity_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 14202000,
                processing_fee: 1098260,
                ..Default::default()
            };

            do_should_add_one_new_key_to_identity(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_add_one_new_key_to_identity_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            // Adjust the expected processing_fee if it differs from "apply = true"
            // or as needed for your scenario
            let expected_fee_result = FeeResult {
                storage_fee: 17145000,
                processing_fee: 5483620,
                ..Default::default()
            };

            do_should_add_one_new_key_to_identity(false, platform_version, expected_fee_result);
        }

        #[test]
        fn should_add_one_new_key_to_identity_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 14202000,
                // 2 extra loaded bytes because the token tree is no longer empty
                // these 2 loaded bytes cost 20 credits each
                processing_fee: 1098300,
                ..Default::default()
            };

            do_should_add_one_new_key_to_identity(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_add_one_new_key_to_identity_latest_version_estimated() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 17145000,
                processing_fee: 5483620,
                ..Default::default()
            };

            do_should_add_one_new_key_to_identity(false, platform_version, expected_fee_result);
        }

        fn do_should_add_one_new_key_to_identity(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            // We assume the user wants to apply or just estimate for adding identity
            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block,
                    apply,
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
                    apply,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("expected to update identity with new keys");

            assert_eq!(fee_result, expected_fee_result);

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to be able to commit a transaction");

                let identity_keys = drive
                    .fetch_all_identity_keys(identity.id().to_buffer(), None, platform_version)
                    .expect("expected to get keys");
                assert_eq!(identity_keys.len(), 6); // we had 5 keys and we added 1
            } else {
                // Not applying -> no commit. We can check root hash if we want
                let app_hash_after = drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .expect("should return app hash");
                // Or any other logic to ensure no state changes actually took effect
                let _ = app_hash_after;
            }
        }

        // -------------------------------------------------
        // check_reference_below_tokens_cost (4 tests)
        // This test exists to make sure the update cost that goes through the tokens tree
        // (as key hash references are below the token tree)
        // stay the same cost.
        // -------------------------------------------------
        #[test]
        fn check_reference_below_tokens_cost_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                processing_fee: 406100,
                ..Default::default()
            };
            do_check_reference_below_tokens_cost(true, platform_version, expected_fee_result);
        }

        #[test]
        fn check_reference_below_tokens_cost_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                processing_fee: 314560,
                ..Default::default()
            };
            do_check_reference_below_tokens_cost(false, platform_version, expected_fee_result);
        }

        #[test]
        fn check_reference_below_tokens_cost_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                // 2 extra loaded bytes because the token tree is no longer empty
                // these 2 loaded bytes cost 20 credits each
                processing_fee: 406140,
                ..Default::default()
            };
            do_check_reference_below_tokens_cost(true, platform_version, expected_fee_result);
        }

        #[test]
        fn check_reference_below_tokens_cost_latest_version_estimated() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                // 2 extra loaded bytes because the token tree is no longer empty
                // these 2 loaded bytes cost 20 credits each
                processing_fee: 314600,
                ..Default::default()
            };
            do_check_reference_below_tokens_cost(false, platform_version, expected_fee_result);
        }

        fn do_check_reference_below_tokens_cost(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block,
                    apply,
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

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to be able to commit a transaction");
            } else {
                // Not applying -> no commit
            }
        }

        // -------------------------------------------------
        // should_add_two_dozen_new_keys_to_identity (4 tests)
        // -------------------------------------------------
        #[test]
        fn should_add_two_dozen_new_keys_to_identity_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 347382000,
                processing_fee: 6819220,
                ..Default::default()
            };

            do_should_add_two_dozen_new_keys_to_identity(
                true,
                platform_version,
                expected_fee_result,
            );
        }

        #[test]
        fn should_add_two_dozen_new_keys_to_identity_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            // Possibly different processing fee if "estimated" differs
            let expected_fee_result = FeeResult {
                storage_fee: 356211000,
                processing_fee: 11699520,
                ..Default::default()
            };

            do_should_add_two_dozen_new_keys_to_identity(
                false,
                platform_version,
                expected_fee_result,
            );
        }

        #[test]
        fn should_add_two_dozen_new_keys_to_identity_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 347382000,
                processing_fee: 6819260,
                ..Default::default()
            };

            do_should_add_two_dozen_new_keys_to_identity(
                true,
                platform_version,
                expected_fee_result,
            );
        }

        #[test]
        fn should_add_two_dozen_new_keys_to_identity_latest_version_estimated() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 356211000,
                processing_fee: 11699520,
                ..Default::default()
            };

            do_should_add_two_dozen_new_keys_to_identity(
                false,
                platform_version,
                expected_fee_result,
            );
        }

        fn do_should_add_two_dozen_new_keys_to_identity(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block,
                    apply,
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
                    apply,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("expected to update identity with new keys");

            assert_eq!(fee_result, expected_fee_result);

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to be able to commit a transaction");

                let identity_keys = drive
                    .fetch_all_identity_keys(identity.id().to_buffer(), None, platform_version)
                    .expect("expected to get balance");

                assert_eq!(identity_keys.len(), 29); // we had 5 keys, added 24
            } else {
                // Not applying => no commit
            }
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

        // -------------------------------------------------
        // should_disable_a_few_keys (4 tests)
        // -------------------------------------------------
        #[test]
        fn should_disable_a_few_keys_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 513000,
                processing_fee: 869380,
                ..Default::default()
            };
            do_should_disable_a_few_keys(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_disable_a_few_keys_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };
            do_should_disable_a_few_keys(false, platform_version, expected_fee_result);
        }

        #[test]
        fn should_disable_a_few_keys_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 513000,
                processing_fee: 869380,
                ..Default::default()
            };
            do_should_disable_a_few_keys(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_disable_a_few_keys_latest_version_estimated() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };
            do_should_disable_a_few_keys(false, platform_version, expected_fee_result);
        }

        fn do_should_disable_a_few_keys(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

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
                    apply,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("should disable a few keys");

            assert_eq!(fee_result, expected_fee_result);

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let identity_keys = drive
                    .fetch_all_identity_keys(identity.id().to_buffer(), None, platform_version)
                    .expect("expected to get balance");

                assert_eq!(identity_keys.len(), 7); // we had 5 keys and we added 2
                for (_, key) in identity_keys.into_iter().skip(5) {
                    assert_eq!(key.disabled_at(), Some(disable_at));
                }
            }
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
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

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

        // -------------------------------------------------
        // should_update_revision (4 tests)
        // -------------------------------------------------
        #[test]
        fn should_update_revision_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 238820,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            do_should_update_revision(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_update_revision_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            // Possibly different if your scenario's estimated cost differs
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 1813560,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            do_should_update_revision(false, platform_version, expected_fee_result);
        }

        #[test]
        fn should_update_revision_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 238820,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            do_should_update_revision(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_update_revision_latest_version_estimated() {
            let platform_version = PlatformVersion::latest();
            // Possibly different if your scenario's estimated cost differs
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 1813560,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            do_should_update_revision(false, platform_version, expected_fee_result);
        }

        fn do_should_update_revision(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");
            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            // Insert identity
            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block_info,
                    apply,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            // We'll update the revision from 0 -> 2
            let revision = 2;

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .update_identity_revision(
                    identity.id().to_buffer(),
                    revision,
                    &block_info,
                    apply,
                    Some(&db_transaction),
                    platform_version,
                    None,
                )
                .expect("should update revision");

            assert_eq!(fee_result, expected_fee_result);

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to be able to commit a transaction");

                // check new revision
                let updated_revision = drive
                    .fetch_identity_revision(
                        identity.id().to_buffer(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to get revision");
                assert_eq!(updated_revision, Some(revision));
            } else {
                // No commit => no changes
            }
        }
    }
}
