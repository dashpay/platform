#[cfg(test)]
mod tests {

    use dpp::prelude::*;

    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::test_utils::identities::create_test_identity;
    use dpp::block::epoch::Epoch;
    use dpp::identity::accessors::IdentityGettersV0;

    mod add_to_identity_balance {
        use dpp::block::block_info::BlockInfo;
        use dpp::fee::fee_result::FeeResult;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::version::PlatformVersion;

        use crate::fees::op::LowLevelDriveOperation;

        use super::*;

        #[test]
        fn should_add_to_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let old_balance = identity.balance();

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

            let db_transaction = drive.grove.start_transaction();

            let amount = 300;

            let fee_result = drive
                .add_to_identity_balance(
                    identity.id().to_buffer(),
                    amount,
                    &block_info,
                    true,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("expected to add to identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 174660,
                    removed_bytes_from_system: 0,
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(
                    identity.id().to_buffer(),
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to get balance");

            assert_eq!(balance.unwrap(), old_balance + amount);
        }

        #[test]
        fn should_fail_if_balance_is_not_persisted() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let result = drive.add_to_identity_balance(
                [0; 32],
                300,
                &block_info,
                true,
                None,
                platform_version,
            );

            assert!(
                matches!(result, Err(Error::Drive(DriveError::CorruptedCodeExecution(m))) if m == "there should always be a balance")
            );
        }

        #[test]
        fn should_deduct_from_debt_if_balance_is_nil() {
            let drive = setup_drive_with_initial_state_structure();
            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(1), None, platform_version)
                .expect("expected an identity");

            let added_balance = 300;
            let negative_amount = 100;

            // Persist negative balance
            let batch = vec![drive
                .update_identity_negative_credit_operation(
                    identity.id().to_buffer(),
                    negative_amount,
                    platform_version,
                )
                .expect("expected to get an update_identity_negative_credit_operation")];

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

            let block_info = BlockInfo::default();

            let fee_result = drive
                .add_to_identity_balance(
                    identity.id().to_buffer(),
                    added_balance,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add to identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 0,
                    processing_fee: 385160,
                    removed_bytes_from_system: 0,
                    ..Default::default()
                }
            );

            let (updated_balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(
                    identity.id().to_buffer(),
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to get balance");

            assert_eq!(
                updated_balance.expect("balance should present"),
                added_balance - negative_amount
            );

            let updated_negative_balance = drive
                .fetch_identity_negative_balance_operations(
                    identity.id().to_buffer(),
                    true,
                    None,
                    &mut drive_operations,
                    platform_version,
                )
                .expect("expected to get balance")
                .expect("balance should present");

            assert_eq!(updated_negative_balance, 0)
        }

        #[test]
        fn should_keep_nil_balance_and_reduce_debt_if_added_balance_is_lower() {
            let drive = setup_drive_with_initial_state_structure();
            let platform_version = PlatformVersion::latest();
            let identity = create_test_identity(&drive, [0; 32], Some(1), None, platform_version)
                .expect("expected an identity");

            let added_balance = 50;
            let negative_amount = 100;

            // Persist negative balance
            let batch = vec![drive
                .update_identity_negative_credit_operation(
                    identity.id().to_buffer(),
                    negative_amount,
                    platform_version,
                )
                .expect("expected to get an update_identity_negative_credit_operation")];

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

            let block_info = BlockInfo::default();

            let fee_result = drive
                .add_to_identity_balance(
                    identity.id().to_buffer(),
                    added_balance,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add to identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    storage_fee: 0,
                    processing_fee: 260540,
                    removed_bytes_from_system: 0,
                    ..Default::default()
                }
            );

            let (updated_balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(
                    identity.id().to_buffer(),
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to get balance");

            assert_eq!(updated_balance.expect("balance should present"), 0);

            let updated_negative_balance = drive
                .fetch_identity_negative_balance_operations(
                    identity.id().to_buffer(),
                    true,
                    None,
                    &mut drive_operations,
                    platform_version,
                )
                .expect("expected to get balance")
                .expect("balance should present");

            assert_eq!(updated_negative_balance, negative_amount - added_balance)
        }

        #[test]
        fn should_estimate_costs_without_state() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let app_hash_before = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            let fee_result = drive
                .add_to_identity_balance(
                    identity.id().to_buffer(),
                    300,
                    &block,
                    false,
                    None,
                    platform_version,
                )
                .expect("expected to get estimated costs to update an identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 4278840,
                    ..Default::default()
                }
            );

            let app_hash_after = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(
                    identity.id().to_buffer(),
                    &block,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to get balance");

            assert!(balance.is_none()); //shouldn't have changed
        }
    }

    mod remove_from_identity_balance {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::fee::fee_result::FeeResult;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_remove_from_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let old_balance = identity.balance();

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

            let db_transaction = drive.grove.start_transaction();

            let amount = 10;

            let fee_result = drive
                .remove_from_identity_balance(
                    identity.id().to_buffer(),
                    amount,
                    &block,
                    true,
                    Some(&db_transaction),
                    platform_version,
                    None,
                )
                .expect("expected to add to identity balance");

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 174660,
                    removed_bytes_from_system: 0,
                    ..Default::default()
                }
            );

            drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(
                    identity.id().to_buffer(),
                    &block,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to get balance");

            assert_eq!(balance.unwrap(), old_balance - amount);
        }

        #[test]
        fn should_estimated_costs_without_state() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let app_hash_before = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            let amount = 10;

            let fee_result = drive
                .remove_from_identity_balance(
                    identity.id().to_buffer(),
                    amount,
                    &block,
                    false,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to add to identity balance");

            let app_hash_after = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            assert_eq!(app_hash_after, app_hash_before);

            assert_eq!(
                fee_result,
                FeeResult {
                    processing_fee: 2476860,
                    ..Default::default()
                }
            );

            let (balance, _fee_cost) = drive
                .fetch_identity_balance_with_costs(
                    identity.id().to_buffer(),
                    &block,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to get balance");

            assert!(balance.is_none()); //shouldn't have changed
        }
    }

    mod apply_balance_change_from_fee_to_identity_operations {
        use super::*;
        use crate::error::identity::IdentityError;
        use crate::fees::op::LowLevelDriveOperation;
        use dpp::block::block_info::BlockInfo;
        use dpp::fee::epoch::{CreditsPerEpoch, GENESIS_EPOCH_INDEX};
        use dpp::fee::fee_result::refunds::{CreditsPerEpochByIdentifier, FeeRefunds};
        use dpp::fee::fee_result::FeeResult;
        use dpp::fee::{Credits, SignedCredits};
        use dpp::version::PlatformVersion;
        use grovedb::batch::GroveOp;
        use grovedb::Element;
        use nohash_hasher::IntMap;
        use std::collections::BTreeMap;

        #[test]
        fn should_do_nothing_if_there_is_no_balance_change() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None, platform_version)
                .expect("expected to create an identity");

            let fee_result = FeeResult::default_with_fees(0, 0);
            let fee_change = fee_result.clone().into_balance_change(identity.id());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(
                    fee_change,
                    None,
                    platform_version,
                )
                .expect("should apply fee change");

            assert_eq!(drive_operations.len(), 0);
            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_add_to_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None, platform_version)
                .expect("expected to create an identity");
            let other_identity =
                create_test_identity(&drive, [1; 32], Some(16), None, platform_version)
                    .expect("expected to create an identity");

            let removed_credits = 100000;
            let other_removed_credits = 200000;

            let credits_per_epoch: CreditsPerEpoch =
                IntMap::from_iter([(GENESIS_EPOCH_INDEX, removed_credits)]);

            let other_credits_per_epoch: CreditsPerEpoch =
                IntMap::from_iter([(GENESIS_EPOCH_INDEX, other_removed_credits)]);

            let refunds_per_epoch_by_identifier: CreditsPerEpochByIdentifier =
                BTreeMap::from_iter([
                    (identity.id().to_buffer(), credits_per_epoch),
                    (other_identity.id().to_buffer(), other_credits_per_epoch),
                ]);

            let fee_result = FeeResult {
                fee_refunds: FeeRefunds(refunds_per_epoch_by_identifier),
                ..Default::default()
            };
            let fee_change = fee_result.clone().into_balance_change(identity.id());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(
                    fee_change,
                    None,
                    platform_version,
                )
                .expect("should apply fee change");

            assert!(matches!(
                drive_operations[..],
                [
                    _,
                    _,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::QualifiedGroveDbOp {
                        op: GroveOp::Replace {
                            element: Element::SumItem(refund_amount, None),
                        },
                        ..
                    }),
                    ..,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::QualifiedGroveDbOp {
                        op: GroveOp::Replace {
                            element: Element::SumItem(other_refund_amount, None),
                        },
                        ..
                    })
                ] if refund_amount as Credits == removed_credits && other_refund_amount as Credits == other_removed_credits
            ));

            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_fail_if_balance_is_not_persisted() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let fee_result = FeeResult::default_with_fees(100000, 100);
            let fee_change = fee_result.into_balance_change([0; 32].into());

            let result = drive.apply_balance_change_from_fee_to_identity_operations(
                fee_change,
                None,
                platform_version,
            );

            assert!(
                matches!(result, Err(Error::Drive(DriveError::CorruptedCodeExecution(m))) if m == "there should always be a balance if apply is set to true")
            );
        }

        #[test]
        fn should_deduct_from_debt_if_balance_is_nil() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let removed_credits = 10000;
            let negative_amount = 1000;

            let identity = create_test_identity(&drive, [0; 32], Some(15), None, platform_version)
                .expect("expected to create an identity");

            // Persist negative balance
            let batch = vec![drive
                .update_identity_negative_credit_operation(
                    identity.id().to_buffer(),
                    negative_amount,
                    platform_version,
                )
                .expect("expected to get an update_identity_negative_credit_operation")];

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

            let credits_per_epoch: CreditsPerEpoch =
                IntMap::from_iter([(GENESIS_EPOCH_INDEX, removed_credits)]);

            let refunds_per_epoch_by_identifier: CreditsPerEpochByIdentifier =
                BTreeMap::from_iter([(identity.id().to_buffer(), credits_per_epoch)]);

            let fee_result = FeeResult {
                fee_refunds: FeeRefunds(refunds_per_epoch_by_identifier),
                ..Default::default()
            };
            let fee_change = fee_result.clone().into_balance_change(identity.id());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(
                    fee_change,
                    None,
                    platform_version,
                )
                .expect("should apply fee change");

            assert!(matches!(
                &drive_operations[..],
                [
                    _,
                    _,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::QualifiedGroveDbOp {
                        op: GroveOp::Replace {
                            element: Element::SumItem(refund_amount, None),
                        },
                    ..
                    }),
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::QualifiedGroveDbOp {
                        op: GroveOp::Replace {
                            element: Element::Item(debt_bytes, None),
                        },
                        ..
                    })
                ] if *refund_amount as Credits == removed_credits - negative_amount && debt_bytes == &0u64.to_be_bytes()
            ));

            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_keep_nil_balance_and_reduce_debt_if_added_balance_is_lower() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let removed_credits = 1000;
            let negative_amount = 3000;

            let identity = create_test_identity(&drive, [0; 32], Some(15), None, platform_version)
                .expect("expected to create an identity");

            // Persist negative balance
            let batch = vec![drive
                .update_identity_negative_credit_operation(
                    identity.id().to_buffer(),
                    negative_amount,
                    platform_version,
                )
                .expect("expected to get an update_identity_negative_credit_operation")];

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

            let credits_per_epoch: CreditsPerEpoch =
                IntMap::from_iter([(GENESIS_EPOCH_INDEX, removed_credits)]);

            let refunds_per_epoch_by_identifier: CreditsPerEpochByIdentifier =
                BTreeMap::from_iter([(identity.id().to_buffer(), credits_per_epoch)]);

            let fee_result = FeeResult {
                fee_refunds: FeeRefunds(refunds_per_epoch_by_identifier),
                ..Default::default()
            };
            let fee_change = fee_result.clone().into_balance_change(identity.id());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(
                    fee_change,
                    None,
                    platform_version,
                )
                .expect("should apply fee change");

            assert!(matches!(
                &drive_operations[..],
                [
                    _,
                    _,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::QualifiedGroveDbOp {
                        op: GroveOp::Replace {
                            element: Element::Item(debt_bytes, None),
                        },
                        ..
                    })
                ] if debt_bytes == &2000u64.to_be_bytes()
            ));

            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_remove_from_balance_less_amount() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None, platform_version)
                .expect("expected to create an identity");

            let initial_balance = 100;

            drive
                .add_to_identity_balance(
                    identity.id().to_buffer(),
                    initial_balance,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("should set initial balance");

            let processing_fee = 20;
            let storage_fee = 10;

            let fee_result = FeeResult {
                processing_fee,
                storage_fee,
                ..Default::default()
            };

            let fee_change = fee_result.clone().into_balance_change(identity.id());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(
                    fee_change,
                    None,
                    platform_version,
                )
                .expect("should apply fee change");

            assert!(matches!(
                drive_operations[..],
                [_, LowLevelDriveOperation::GroveOperation(grovedb::batch::QualifiedGroveDbOp {
                    op: GroveOp::Replace {
                        element: Element::SumItem(balance, None),
                    },
                    ..
                })] if balance == (initial_balance - storage_fee - processing_fee) as SignedCredits
            ));

            assert_eq!(fee_result_outcome, fee_result);
        }

        #[test]
        fn should_remove_from_balance_bigger_amount_and_get_into_debt() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None, platform_version)
                .expect("expected to create an identity");

            let initial_balance = 100;

            drive
                .add_to_identity_balance(
                    identity.id().to_buffer(),
                    initial_balance,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("should set initial balance");

            let processing_fee = 110;
            let storage_fee = 80;

            let fee_result = FeeResult {
                processing_fee,
                storage_fee,
                ..Default::default()
            };

            let fee_change = fee_result.into_balance_change(identity.id());

            let (drive_operations, fee_result_outcome) = drive
                .apply_balance_change_from_fee_to_identity_operations(
                    fee_change,
                    None,
                    platform_version,
                )
                .expect("should apply fee change");

            let expected_debt_bytes =
                (storage_fee + processing_fee - initial_balance).to_be_bytes();

            assert!(matches!(
                &drive_operations[..],
                [
                    _,
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::QualifiedGroveDbOp {
                        op: GroveOp::Replace {
                            element: Element::SumItem(balance, None),
                        },
                        ..
                    }),
                    LowLevelDriveOperation::GroveOperation(grovedb::batch::QualifiedGroveDbOp {
                        op: GroveOp::Replace {
                            element: Element::Item(debt_bytes, None),
                        },
                        ..
                    })
                ] if balance == &(0 as SignedCredits) && debt_bytes == &expected_debt_bytes
            ));

            assert_eq!(
                fee_result_outcome,
                FeeResult {
                    storage_fee,
                    processing_fee: initial_balance - storage_fee,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn should_return_error_if_required_amount_bigger_than_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = create_test_identity(&drive, [0; 32], Some(15), None, platform_version)
                .expect("expected to create an identity");

            let processing_fee = 110;
            let storage_fee = 80;

            let fee_result = FeeResult {
                processing_fee,
                storage_fee,
                ..Default::default()
            };

            let fee_change = fee_result.into_balance_change(identity.id());

            let result = drive.apply_balance_change_from_fee_to_identity_operations(
                fee_change,
                None,
                platform_version,
            );

            assert!(matches!(
                result,
                Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                    _
                )))
            ));
        }
    }
}
