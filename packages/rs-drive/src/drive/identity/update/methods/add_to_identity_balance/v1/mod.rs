use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id.
    ///
    /// Generation 1 (protocol version 14) is generation 0, and the credits that repay the
    /// identity's debt reach the processing fee pool of the block's epoch, applied after the
    /// balance write and not billed, so the fee is generation 0's. Without a caller transaction
    /// both writes share one owned transaction, committed only once the pool write applied, so
    /// the debt is never cleared without the pool receiving what repaid it.
    #[inline(always)]
    pub(super) fn add_to_identity_balance_v1(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let caller_transaction = transaction;
        let owned_transaction =
            (apply && caller_transaction.is_none()).then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(caller_transaction);

        let mut batch_operations = self.add_to_identity_balance_operations_v1(
            identity_id,
            added_balance,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        let repaid_debt = LowLevelDriveOperation::take_repaid_identity_debt(&mut batch_operations)?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // A dry run reads no debt, so it never repays one
        self.apply_repaid_identity_debt_to_processing_pool(
            repaid_debt,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }

        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;

        Ok(fees)
    }

    /// Balances are stored in the balance tree under the identity's id
    /// This gets operations based on apply flag (stateful vs stateless)
    ///
    /// Generation 1 (protocol version 14) is generation 0, and the operations end with a
    /// [`LowLevelDriveOperation::RepaidIdentityDebt`] when the added credits repaid the
    /// identity's debt: the identity's balance gets only what is left, as in generation 0, and
    /// whoever applies the operations owes the repaid part to the current epoch's processing
    /// fee pool, where the unpaid processing fees the debt stood for would have gone. In
    /// generation 0 that part reached no balance the credit sum counts.
    #[inline(always)]
    pub(super) fn add_to_identity_balance_operations_v1(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let (mut drive_operations, repaid_debt) = self
            .add_to_identity_balance_operations_and_repaid_debt_v0(
                identity_id,
                added_balance,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;

        if repaid_debt > 0 {
            drive_operations.push(LowLevelDriveOperation::RepaidIdentityDebt(repaid_debt));
        }

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use crate::drive::identity::update::methods::debt_test_helpers::{
        balance, credits_are_balanced, debt, debt_test_block_info as block_info, indebted_identity,
        processing_pool, DEBT_TEST_EPOCH_INDEX as EPOCH_INDEX,
    };
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::batch::{DriveOperation, IdentityOperationType, SystemOperationType};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::test_utils::identities::create_test_identity;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_credit_the_repaid_debt_to_the_processing_fee_pool_and_keep_the_credit_sum() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = indebted_identity(&drive, [1; 32], 100, platform_version);
        assert!(credits_are_balanced(&drive, platform_version));

        // A top-up: the credits enter the system and reach the identity
        drive
            .add_to_system_credits(300, None, platform_version)
            .expect("expected to add system credits");
        drive
            .add_to_identity_balance(
                identity_id,
                300,
                &block_info(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add to the balance");

        assert_eq!(balance(&drive, identity_id, platform_version), 200);
        assert_eq!(debt(&drive, identity_id, platform_version), 0);
        assert_eq!(processing_pool(&drive, platform_version), 100);
        assert!(credits_are_balanced(&drive, platform_version));
    }

    #[test]
    fn should_credit_all_added_credits_to_the_pool_when_they_do_not_cover_the_debt() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = indebted_identity(&drive, [1; 32], 100, platform_version);

        drive
            .add_to_system_credits(60, None, platform_version)
            .expect("expected to add system credits");
        drive
            .add_to_identity_balance(identity_id, 60, &block_info(), true, None, platform_version)
            .expect("expected to add to the balance");

        assert_eq!(balance(&drive, identity_id, platform_version), 0);
        assert_eq!(debt(&drive, identity_id, platform_version), 40);
        assert_eq!(processing_pool(&drive, platform_version), 60);
        assert!(credits_are_balanced(&drive, platform_version));
    }

    #[test]
    fn should_leave_the_repaid_debt_in_no_counted_balance_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let identity_id = indebted_identity(&drive, [1; 32], 100, platform_version);

        drive
            .add_to_system_credits(300, None, platform_version)
            .expect("expected to add system credits");
        drive
            .add_to_identity_balance(
                identity_id,
                300,
                &block_info(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add to the balance");

        // Generation 0: the identity still repays, and the repaid part reaches no pool
        assert_eq!(balance(&drive, identity_id, platform_version), 200);
        assert_eq!(debt(&drive, identity_id, platform_version), 0);
        assert_eq!(processing_pool(&drive, platform_version), 0);
        assert!(!credits_are_balanced(&drive, platform_version));
    }

    #[test]
    fn should_credit_every_debt_one_batch_repays_to_the_processing_fee_pool_once() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let first = indebted_identity(&drive, [1; 32], 100, platform_version);
        let second = indebted_identity(&drive, [2; 32], 50, platform_version);

        drive
            .apply_drive_operations(
                vec![
                    DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits {
                        amount: 500,
                    }),
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id: first,
                            added_balance: 300,
                        },
                    ),
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id: second,
                            added_balance: 200,
                        },
                    ),
                ],
                true,
                &block_info(),
                None,
                platform_version,
                None,
            )
            .expect("expected to apply the batch");

        assert_eq!(balance(&drive, first, platform_version), 200);
        assert_eq!(balance(&drive, second, platform_version), 150);
        assert_eq!(processing_pool(&drive, platform_version), 150);
        assert!(credits_are_balanced(&drive, platform_version));
    }

    #[test]
    fn should_add_the_repaid_debt_to_a_processing_fee_pool_write_of_the_same_batch() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = indebted_identity(&drive, [1; 32], 100, platform_version);
        let epoch = Epoch::new(EPOCH_INDEX).expect("a valid epoch index");

        // The end of a block writes the epoch's processing fees in the batch that also pays
        // the epoch's proposers: the repaid part must add to that write, not race it
        drive
            .apply_drive_operations(
                vec![
                    DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits {
                        amount: 1300,
                    }),
                    DriveOperation::GroveDBOperation(
                        epoch
                            .update_processing_fee_pool_operation(1000)
                            .expect("expected a pool operation"),
                    ),
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id,
                            added_balance: 300,
                        },
                    ),
                ],
                true,
                &block_info(),
                None,
                platform_version,
                None,
            )
            .expect("expected to apply the batch");

        assert_eq!(balance(&drive, identity_id, platform_version), 200);
        assert_eq!(processing_pool(&drive, platform_version), 1100);
        assert!(credits_are_balanced(&drive, platform_version));
    }

    #[test]
    fn should_repay_nothing_when_only_estimating() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = indebted_identity(&drive, [1; 32], 100, platform_version);

        let mut estimated_costs_only_with_layer_info = Some(Default::default());
        let operations = drive
            .add_to_identity_balance_operations(
                identity_id,
                300,
                &mut estimated_costs_only_with_layer_info,
                None,
                platform_version,
            )
            .expect("expected estimated operations");

        assert!(!LowLevelDriveOperation::holds_repaid_identity_debt(
            &operations
        ));
    }

    #[test]
    fn should_refuse_to_apply_a_batch_holding_an_unrouted_repaid_debt() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);

        let result = drive.apply_batch_low_level_drive_operations(
            None,
            None,
            vec![LowLevelDriveOperation::RepaidIdentityDebt(1)],
            &mut vec![],
            &platform_version.drive,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ));
    }

    #[test]
    fn should_refuse_to_convert_a_debt_repaying_credit_into_a_plain_batch() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = indebted_identity(&drive, [1; 32], 100, platform_version);

        let result = drive.convert_drive_operations_to_grove_operations(
            vec![DriveOperation::IdentityOperation(
                IdentityOperationType::AddToIdentityBalance {
                    identity_id,
                    added_balance: 300,
                },
            )],
            &block_info(),
            None,
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ));
    }

    #[test]
    fn should_repay_a_debt_once_when_one_batch_credits_the_identity_twice() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = indebted_identity(&drive, [1; 32], 100, platform_version);

        // Two credits of 60 against a debt of 100: each converted alone would repay the same
        // debt and keep only one balance write
        drive
            .apply_drive_operations(
                vec![
                    DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits {
                        amount: 120,
                    }),
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id,
                            added_balance: 60,
                        },
                    ),
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id,
                            added_balance: 60,
                        },
                    ),
                ],
                true,
                &block_info(),
                None,
                platform_version,
                None,
            )
            .expect("expected to apply the batch");

        assert_eq!(balance(&drive, identity_id, platform_version), 20);
        assert_eq!(debt(&drive, identity_id, platform_version), 0);
        assert_eq!(processing_pool(&drive, platform_version), 100);
        assert!(credits_are_balanced(&drive, platform_version));
    }

    #[test]
    fn should_merge_a_credit_and_a_debit_of_one_indebted_identity_into_one_net_write() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = indebted_identity(&drive, [1; 32], 100, platform_version);

        // A credit of 300 and a debit of 7 in one batch: each converted alone would read the
        // committed balance, and one balance write would replace the other
        drive
            .apply_drive_operations(
                vec![
                    DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits {
                        amount: 293,
                    }),
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id,
                            added_balance: 300,
                        },
                    ),
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::RemoveFromIdentityBalance {
                            identity_id,
                            balance_to_remove: 7,
                        },
                    ),
                ],
                true,
                &block_info(),
                None,
                platform_version,
                None,
            )
            .expect("expected to apply the batch");

        assert_eq!(balance(&drive, identity_id, platform_version), 193);
        assert_eq!(debt(&drive, identity_id, platform_version), 0);
        assert_eq!(processing_pool(&drive, platform_version), 100);
        assert!(credits_are_balanced(&drive, platform_version));
    }

    #[test]
    fn should_keep_every_credit_one_batch_makes_to_one_identity() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let identity = create_test_identity(&drive, [1; 32], Some(1), None, platform_version)
            .expect("expected an identity");
        let identity_id = identity.id().to_buffer();

        drive
            .apply_drive_operations(
                vec![
                    DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits {
                        amount: 120,
                    }),
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id,
                            added_balance: 50,
                        },
                    ),
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id,
                            added_balance: 70,
                        },
                    ),
                ],
                true,
                &block_info(),
                None,
                platform_version,
                None,
            )
            .expect("expected to apply the batch");

        assert_eq!(balance(&drive, identity_id, platform_version), 120);
        assert!(credits_are_balanced(&drive, platform_version));
    }

    #[test]
    fn should_commit_neither_write_when_the_pool_write_fails_without_a_caller_transaction() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = indebted_identity(&drive, [1; 32], 100, platform_version);

        // No pool tree exists for this epoch, so the pool write fails after the balance write
        // applied in the method's own transaction
        let missing_epoch = Epoch::new(10_000).expect("a valid epoch index");
        drive
            .add_to_identity_balance(
                identity_id,
                300,
                &BlockInfo::default_with_epoch(missing_epoch),
                true,
                None,
                platform_version,
            )
            .expect_err("the pool write should fail");

        assert_eq!(balance(&drive, identity_id, platform_version), 0);
        assert_eq!(debt(&drive, identity_id, platform_version), 100);
    }
}
