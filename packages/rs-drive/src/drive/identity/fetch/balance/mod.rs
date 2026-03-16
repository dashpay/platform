mod fetch_identity_balance;
mod fetch_identity_balance_include_debt;
mod fetch_identity_negative_balance;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::test_utils::identities::create_test_identity;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    mod fetch_identity_balance {
        use super::*;

        #[test]
        fn should_return_none_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let balance = drive
                .fetch_identity_balance([0; 32], None, platform_version)
                .expect("should not error");

            assert!(balance.is_none());
        }

        #[test]
        fn should_return_balance_for_existing_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(42), platform_version)
                .expect("expected a random identity");

            let expected_balance = identity.balance();

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

            let balance = drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("should not error")
                .expect("should have balance");

            assert_eq!(balance, expected_balance);
        }

        #[test]
        fn should_return_balance_with_costs_estimated() {
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

            let block_info = BlockInfo::default();

            // Estimated mode (apply=false)
            let (balance, fee_result) = drive
                .fetch_identity_balance_with_costs(
                    identity.id().to_buffer(),
                    &block_info,
                    false,
                    None,
                    platform_version,
                )
                .expect("should return balance with costs");

            // In estimated (stateless) mode, the balance query does not hit the
            // database at all. Production returns Some(0) as a placeholder value
            // rather than the real balance; the purpose is purely to compute
            // estimated operation costs. This is expected behavior, not a real
            // balance result.
            assert!(fee_result.processing_fee > 0);
            assert_eq!(
                balance,
                Some(0),
                "estimated mode should return placeholder balance 0, not the real balance"
            );
        }
    }

    mod fetch_identity_balance_include_debt {
        use super::*;
        use crate::fees::op::LowLevelDriveOperation;

        #[test]
        fn should_return_none_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let balance = drive
                .fetch_identity_balance_include_debt([0; 32], None, platform_version)
                .expect("should not error");

            assert!(balance.is_none());
        }

        #[test]
        fn should_return_positive_balance_for_identity_with_funds() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(1), None, platform_version)
                .expect("expected an identity");

            let added_balance = 1000;

            drive
                .add_to_identity_balance(
                    identity.id().to_buffer(),
                    added_balance,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("should add balance");

            let balance = drive
                .fetch_identity_balance_include_debt(
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("should not error")
                .expect("should have balance");

            assert_eq!(balance, added_balance as i64);
        }

        #[test]
        fn should_return_negative_balance_for_identity_with_debt() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(1), None, platform_version)
                .expect("expected an identity");

            let negative_amount: u64 = 500;

            // Set negative balance (debt)
            let batch = vec![drive
                .update_identity_negative_credit_operation(
                    identity.id().to_buffer(),
                    negative_amount,
                    platform_version,
                )
                .expect("expected operation")];

            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    None,
                    batch,
                    &mut drive_operations,
                    &platform_version.drive,
                )
                .expect("should apply batch");

            let balance = drive
                .fetch_identity_balance_include_debt(
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("should not error")
                .expect("should have balance");

            assert_eq!(balance, -(negative_amount as i64));
        }
    }

    mod fetch_identity_balance_include_debt_combined {
        use super::*;
        use crate::fees::op::LowLevelDriveOperation;

        #[test]
        fn should_return_positive_balance_when_identity_has_both_balance_and_negative_credit() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(1), None, platform_version)
                .expect("expected an identity");

            let added_balance: u64 = 5000;

            // Give the identity a positive balance
            drive
                .add_to_identity_balance(
                    identity.id().to_buffer(),
                    added_balance,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("should add balance");

            // Also set a negative credit (debt) on the same identity
            let negative_amount: u64 = 300;
            let batch = vec![drive
                .update_identity_negative_credit_operation(
                    identity.id().to_buffer(),
                    negative_amount,
                    platform_version,
                )
                .expect("expected operation")];

            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    None,
                    batch,
                    &mut drive_operations,
                    &platform_version.drive,
                )
                .expect("should apply batch");

            // When balance > 0, fetch_identity_balance_include_debt returns the
            // positive balance and does NOT subtract the negative credit.
            let balance = drive
                .fetch_identity_balance_include_debt(
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("should not error")
                .expect("should have balance");

            assert_eq!(
                balance, added_balance as i64,
                "when balance is positive, the negative credit is not consulted"
            );
        }
    }

    mod fetch_identity_negative_balance {
        use super::*;
        use crate::error::Error;

        #[test]
        fn should_error_with_path_parent_layer_not_found_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let mut drive_operations = vec![];
            // For a non-existent identity, the path doesn't exist, so grove_get_raw
            // returns a PathParentLayerNotFound error since the identity subtree
            // doesn't exist at all.
            let result = drive.fetch_identity_negative_balance_operations(
                [0; 32],
                true,
                None,
                &mut drive_operations,
                platform_version,
            );

            assert!(
                matches!(
                    &result,
                    Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathParentLayerNotFound(_))
                ),
                "expected PathParentLayerNotFound error, got: {:?}",
                result
            );
        }

        #[test]
        fn should_return_zero_negative_balance_for_new_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(1), None, platform_version)
                .expect("expected an identity");

            let mut drive_operations = vec![];
            let negative_balance = drive
                .fetch_identity_negative_balance_operations(
                    identity.id().to_buffer(),
                    true,
                    None,
                    &mut drive_operations,
                    platform_version,
                )
                .expect("should not error")
                .expect("should have negative balance");

            assert_eq!(negative_balance, 0);
        }
    }
}
