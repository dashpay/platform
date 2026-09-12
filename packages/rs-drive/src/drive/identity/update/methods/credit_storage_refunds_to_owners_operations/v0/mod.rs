use crate::drive::identity::update::storage_refund_credit_outcome::StorageRefundCreditOutcome;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::fee_result::refunds::FeeRefunds;
use dpp::fee::Credits;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::{BTreeMap, HashMap};

impl Drive {
    /// Credits each recorded refund owner that has a balance element and
    /// reports the rest as routed to the processing pool. See the dispatcher.
    #[inline(always)]
    pub(super) fn credit_storage_refunds_to_owners_operations_v0(
        &self,
        fee_refunds: &FeeRefunds,
        skip_owner: Option<[u8; 32]>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<StorageRefundCreditOutcome, Error> {
        let mut credited: BTreeMap<Identifier, Credits> = BTreeMap::new();
        let mut routed_to_processing_pool: Credits = 0;

        for (owner_id, credits_per_epoch) in fee_refunds.iter() {
            if skip_owner.as_ref() == Some(owner_id) {
                continue;
            }

            let credits = credits_per_epoch
                .values()
                .try_fold(0u64, |sum, epoch_credits| sum.checked_add(*epoch_credits))
                .ok_or(ProtocolError::Overflow(
                    "storage refund credits for one owner overflow",
                ))?;

            if credits == 0 {
                continue;
            }

            // A stateful read: `None` means the balance element does not exist, which
            // is the only signal Drive has today that the owner is gone.
            let existing_balance = self.fetch_identity_balance_operations(
                *owner_id,
                true,
                transaction,
                drive_operations,
                platform_version,
            )?;

            if existing_balance.is_some() {
                let mut estimated_costs_only_with_layer_info =
                    None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>;

                drive_operations.extend(self.add_to_identity_balance_operations(
                    *owner_id,
                    credits,
                    &mut estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?);

                credited.insert(Identifier::from(*owner_id), credits);
            } else {
                routed_to_processing_pool = routed_to_processing_pool.checked_add(credits).ok_or(
                    ProtocolError::Overflow(
                        "storage refund credits routed to the processing pool overflow",
                    ),
                )?;
            }
        }

        Ok(StorageRefundCreditOutcome {
            credited,
            routed_to_processing_pool,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use crate::util::batch::DriveOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::fee::epoch::CreditsPerEpoch;
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::Identity;
    use grovedb::Transaction;

    const IDENTITY_BALANCE: Credits = 10_000_000;
    const PROCESSING_POOL_SEED: Credits = 1_000_000;

    fn insert_identity(
        drive: &Drive,
        seed: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Identity {
        let mut identity = Identity::random_identity(3, Some(seed), platform_version)
            .expect("expected a random identity");
        identity.set_balance(IDENTITY_BALANCE);
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(transaction),
                platform_version,
            )
            .expect("expected to insert the identity");
        identity
    }

    fn refunds(entries: &[([u8; 32], &[(u16, Credits)])]) -> FeeRefunds {
        let mut fee_refunds = FeeRefunds::default();
        for (owner, credits_per_epoch) in entries {
            let mut epochs = CreditsPerEpoch::default();
            for (epoch_index, credits) in credits_per_epoch.iter() {
                epochs.insert(*epoch_index, *credits);
            }
            fee_refunds.0.insert(*owner, epochs);
        }
        fee_refunds
    }

    fn apply(
        drive: &Drive,
        operations: Vec<LowLevelDriveOperation>,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) {
        drive
            .apply_batch_low_level_drive_operations(
                None,
                Some(transaction),
                operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to apply the operations");
    }

    fn balance(
        drive: &Drive,
        identity_id: [u8; 32],
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Option<Credits> {
        drive
            .fetch_identity_balance(identity_id, Some(transaction), platform_version)
            .expect("expected to fetch the balance")
    }

    #[test]
    fn should_credit_each_recorded_owner_by_the_sum_of_its_epochs() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let first = insert_identity(&drive, 1, &transaction, platform_version);
        let second = insert_identity(&drive, 2, &transaction, platform_version);
        let first_id = first.id().to_buffer();
        let second_id = second.id().to_buffer();

        let fee_refunds = refunds(&[
            (first_id, &[(0, 300), (4, 700), (9, 1)]),
            (second_id, &[(2, 5_000)]),
        ]);

        let mut operations = vec![];
        let outcome = drive
            .credit_storage_refunds_to_owners_operations(
                &fee_refunds,
                None,
                Some(&transaction),
                &mut operations,
                platform_version,
            )
            .expect("expected to credit the owners");
        apply(&drive, operations, &transaction, platform_version);

        assert_eq!(
            outcome,
            StorageRefundCreditOutcome {
                credited: BTreeMap::from([(first.id(), 1_001), (second.id(), 5_000)]),
                routed_to_processing_pool: 0,
            }
        );
        assert_eq!(
            balance(&drive, first_id, &transaction, platform_version),
            Some(IDENTITY_BALANCE + 1_001)
        );
        assert_eq!(
            balance(&drive, second_id, &transaction, platform_version),
            Some(IDENTITY_BALANCE + 5_000)
        );
    }

    #[test]
    fn should_report_refunds_for_an_owner_without_a_balance_instead_of_failing() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let existing = insert_identity(&drive, 3, &transaction, platform_version);
        let existing_id = existing.id().to_buffer();
        let missing_id = [0xAB; 32];
        assert_eq!(
            balance(&drive, missing_id, &transaction, platform_version),
            None
        );

        let fee_refunds = refunds(&[
            (existing_id, &[(1, 400)]),
            (missing_id, &[(1, 250), (3, 50)]),
        ]);

        let mut operations = vec![];
        let outcome = drive
            .credit_storage_refunds_to_owners_operations(
                &fee_refunds,
                None,
                Some(&transaction),
                &mut operations,
                platform_version,
            )
            .expect("an owner without a balance is reported, not an error");
        apply(&drive, operations, &transaction, platform_version);

        assert_eq!(
            outcome,
            StorageRefundCreditOutcome {
                credited: BTreeMap::from([(existing.id(), 400)]),
                routed_to_processing_pool: 300,
            }
        );
        assert_eq!(outcome.total(), Some(700));
        assert_eq!(
            balance(&drive, existing_id, &transaction, platform_version),
            Some(IDENTITY_BALANCE + 400)
        );
        assert_eq!(
            balance(&drive, missing_id, &transaction, platform_version),
            None,
            "no balance element is created for the missing owner"
        );
    }

    #[test]
    fn should_credit_an_owner_whose_keys_are_all_disabled() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let frozen = insert_identity(&drive, 4, &transaction, platform_version);
        let frozen_id = frozen.id().to_buffer();
        let key_ids = frozen.public_keys().keys().copied().collect::<Vec<_>>();
        assert_eq!(key_ids.len(), 3);
        drive
            .disable_identity_keys(
                frozen_id,
                key_ids,
                1_000,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to disable every key");

        let fee_refunds = refunds(&[(frozen_id, &[(0, 12_345)])]);

        let mut operations = vec![];
        let outcome = drive
            .credit_storage_refunds_to_owners_operations(
                &fee_refunds,
                None,
                Some(&transaction),
                &mut operations,
                platform_version,
            )
            .expect("no key or permission is consulted");
        apply(&drive, operations, &transaction, platform_version);

        assert_eq!(outcome.credited, BTreeMap::from([(frozen.id(), 12_345)]));
        assert_eq!(outcome.routed_to_processing_pool, 0);
        assert_eq!(
            balance(&drive, frozen_id, &transaction, platform_version),
            Some(IDENTITY_BALANCE + 12_345)
        );
    }

    #[test]
    fn should_skip_the_owner_the_caller_settles_itself() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let payer = insert_identity(&drive, 5, &transaction, platform_version);
        let other = insert_identity(&drive, 6, &transaction, platform_version);
        let payer_id = payer.id().to_buffer();
        let other_id = other.id().to_buffer();

        let fee_refunds = refunds(&[(payer_id, &[(0, 900)]), (other_id, &[(0, 100)])]);

        let mut operations = vec![];
        let outcome = drive
            .credit_storage_refunds_to_owners_operations(
                &fee_refunds,
                Some(payer_id),
                Some(&transaction),
                &mut operations,
                platform_version,
            )
            .expect("expected to credit the other owner");
        apply(&drive, operations, &transaction, platform_version);

        assert_eq!(outcome.credited, BTreeMap::from([(other.id(), 100)]));
        assert_eq!(outcome.routed_to_processing_pool, 0);
        assert_eq!(
            balance(&drive, payer_id, &transaction, platform_version),
            Some(IDENTITY_BALANCE),
            "the payer's refund folds into its own balance change elsewhere"
        );
        assert_eq!(
            balance(&drive, other_id, &transaction, platform_version),
            Some(IDENTITY_BALANCE + 100)
        );
    }

    #[test]
    fn should_leave_total_credits_balanced_after_the_caller_records_the_pending_refunds() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();
        let epoch = Epoch::new(0).expect("epoch 0");

        let credited_owner = insert_identity(&drive, 7, &transaction, platform_version);
        let credited_owner_id = credited_owner.id().to_buffer();
        let missing_owner_id = [0xCD; 32];

        // Every credit in the system is accounted for before the refunds: two
        // identity balances and a seeded processing pool.
        let seed_operation = drive
            .add_epoch_processing_credits_for_distribution_operation(
                &epoch,
                PROCESSING_POOL_SEED,
                Some(&transaction),
                platform_version,
            )
            .expect("expected the pool seed operation");
        apply(&drive, vec![seed_operation], &transaction, platform_version);
        drive
            .add_to_system_credits(
                IDENTITY_BALANCE + PROCESSING_POOL_SEED,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the system credits");
        assert!(drive
            .calculate_total_credits_balance(Some(&transaction), &platform_version.drive)
            .expect("expected the balance")
            .ok()
            .expect("expected a well-formed balance"));

        let fee_refunds = refunds(&[
            (credited_owner_id, &[(0, 100), (1, 50)]),
            (missing_owner_id, &[(0, 70)]),
        ]);

        // The primitive credits the owner that exists and reports the rest.
        let mut operations = vec![];
        let outcome = drive
            .credit_storage_refunds_to_owners_operations(
                &fee_refunds,
                None,
                Some(&transaction),
                &mut operations,
                platform_version,
            )
            .expect("expected to credit the owners");
        assert_eq!(outcome.routed_to_processing_pool, 70);

        // The caller routes the unrouted amount to the epoch's processing pool
        // with one pool write and records every refund against its storage
        // epoch, exactly as a lifecycle settlement does.
        operations.push(
            drive
                .add_epoch_processing_credits_for_distribution_operation(
                    &epoch,
                    outcome.routed_to_processing_pool,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the pool write"),
        );
        apply(&drive, operations, &transaction, platform_version);

        let mut pending_refund_operations: Vec<DriveOperation> = vec![];
        Drive::add_update_pending_epoch_refunds_operations(
            &mut pending_refund_operations,
            fee_refunds.sum_per_epoch(),
            &platform_version.drive,
        )
        .expect("expected the pending refund operations");
        drive
            .apply_drive_operations(
                pending_refund_operations,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to record the pending refunds");

        assert_eq!(
            balance(&drive, credited_owner_id, &transaction, platform_version),
            Some(IDENTITY_BALANCE + 150)
        );
        assert_eq!(
            drive
                .get_epoch_processing_credits_for_distribution(
                    &epoch,
                    Some(&transaction),
                    platform_version
                )
                .expect("expected the pool balance"),
            PROCESSING_POOL_SEED + 70
        );
        assert_eq!(
            drive
                .fetch_pending_epoch_refunds(Some(&transaction), &platform_version.drive)
                .expect("expected the pending refunds"),
            CreditsPerEpoch::from_iter([(0, 170), (1, 50)])
        );
        let total = drive
            .calculate_total_credits_balance(Some(&transaction), &platform_version.drive)
            .expect("expected the balance");
        assert!(
            total.ok().expect("expected a well-formed balance"),
            "credits moved between the pools and an identity balance must stay conserved: {:?}",
            total
        );
    }

    #[test]
    fn should_not_be_active_before_the_new_drive_table() {
        let drive = setup_drive_with_initial_state_structure(None);
        let frozen_platform_version = PlatformVersion::get(14).expect("protocol version 14");
        let transaction = drive.grove.start_transaction();

        let fee_refunds = refunds(&[([1; 32], &[(0, 100)])]);

        let result = drive.credit_storage_refunds_to_owners_operations(
            &fee_refunds,
            None,
            Some(&transaction),
            &mut vec![],
            frozen_platform_version,
        );

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::VersionNotActive { .. }))
            ),
            "protocol version 14 has no lifecycle refund settlement, got {:?}",
            result
        );
    }
}
