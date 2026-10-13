use crate::util::batch::{DriveOperation, SystemOperationType};

use crate::drive::Drive;
use crate::error::{fee::FeeError, Error};
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes;

use crate::util::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use std::collections::HashMap;

impl Drive {
    /// Applies a list of high level DriveOperations to the drive, and calculates the fee for them.
    ///
    /// # Arguments
    ///
    /// * `operations` - A vector of `DriveOperation`s to apply to the drive.
    /// * `apply` - A boolean flag indicating whether to apply the changes or only estimate costs.
    /// * `block_info` - A reference to information about the current block.
    /// * `transaction` - Transaction arguments.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `FeeResult` if the operations are successfully applied,
    /// otherwise an `Error`.
    ///
    /// If `apply` is set to true, it applies the low-level drive operations and updates side info accordingly.
    /// If not, it only estimates the costs and updates estimated costs with layer info.
    ///
    /// Generation 1 (protocol version 14) is generation 0, and a batch that carries a storage
    /// refund forfeiture ([`DriveOperation::forfeits_storage_refunds`], a moderator's document
    /// deletion) refunds nobody for the storage its document operations remove: the document's
    /// own bytes, and those of any index subtree the deletion empties, whoever paid for them
    /// (an earlier author's document may have created it). The bytes still leave the system,
    /// but nobody gets them back, and the credits stay in the storage pools they were
    /// distributed to. When the batch also frees moderation storage someone is owed
    /// ([`DriveOperation::refunds_moderation_storage`]: a restored removal record replaced,
    /// which may shrink, or the approvals and the info of a team action its closing approval
    /// moves), the document operations are applied as a GroveDB batch of their own, after the
    /// rest and in the same transaction, and only that batch forfeits: the moderators who paid
    /// for those keep their refunds. Otherwise nothing but the document operations frees bytes
    /// (a fresh record or approval is an insert, the nonce and the counts keep their size), and
    /// the batch is applied as one, forfeiting whole. An estimate carries no refund to begin
    /// with and prices the batch as it is applied: one GroveDB batch, or two when a forfeiting
    /// deletion also frees moderation storage.
    ///
    /// Every write of one identity balance, one contract fee pot or one prefunded specialized
    /// balance is also merged into one ([`DriveOperation::merge_balance_writes`]): each
    /// computes the new value from the one committed before the batch, so a second write in the
    /// same batch would replace the first and the credits would no longer add up. Token writes
    /// cannot be merged the same way, so a batch that writes one token balance, one token
    /// supply or one token shielded pool total twice is refused
    /// ([`DriveOperation::refuse_repeated_token_balance_writes`]); no state transition makes
    /// one. A batch that writes each key once is applied exactly as by generation 0.
    ///
    /// An estimate is merged the same way, so it prices the batch execution applies. It reads
    /// no balance and lets a merged removal take up to the largest balance there can be; fee
    /// validation estimates for the payer it settles on, and refuses an identity that cannot
    /// fund what it owes before estimating, so no merged removal it estimates takes more.
    ///
    /// Credits the batch adds to an identity that repay its debt
    /// ([`LowLevelDriveOperation::RepaidIdentityDebt`], from `add_to_identity_balance_operations`
    /// 1) go to the processing fee pool of the block's epoch once the batch applied: the debt
    /// stood for processing fees that never reached a pool, and otherwise the credits would
    /// reach no balance the credit sum counts. The pool write reads the state the batch left,
    /// so it adds to a pool write the batch made itself (the fee distribution at the end of a
    /// block) instead of racing it, and it is not billed. An estimate reads no debt and repays
    /// none. The balance writes are merged before the batch is converted, so two credits to an
    /// indebted identity repay its debt once, as one net credit.
    #[inline(always)]
    pub(crate) fn apply_drive_operations_v1(
        &self,
        operations: Vec<DriveOperation>,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        DriveOperation::refuse_repeated_token_balance_writes(&operations)?;
        self.refuse_repeated_counter_moves(&operations, block_info, transaction, platform_version)?;
        let mut operations = DriveOperation::merge_balance_writes(operations)?;
        let mut processing_pool_credits: Credits = 0;
        for operation in operations.extract_if(.., |operation| {
            matches!(
                operation,
                DriveOperation::SystemOperation(
                    SystemOperationType::AddToCurrentEpochProcessingPool { .. }
                )
            )
        }) {
            if let DriveOperation::SystemOperation(
                SystemOperationType::AddToCurrentEpochProcessingPool { amount },
            ) = operation
            {
                processing_pool_credits =
                    processing_pool_credits
                        .checked_add(amount)
                        .ok_or(Error::Fee(FeeError::Overflow(
                            "overflow when accumulating processing pool credits",
                        )))?;
            }
        }
        if operations.is_empty() && (!apply || processing_pool_credits == 0) {
            return Ok(FeeResult::default());
        }
        let forfeits_storage_refunds = operations
            .iter()
            .any(DriveOperation::forfeits_storage_refunds);
        // The document operations of a moderator's deletion go in a batch of their own only
        // when something else in the batch may free bytes someone is owed.
        let separates_forfeited_operations = forfeits_storage_refunds
            && operations
                .iter()
                .any(DriveOperation::refunds_moderation_storage);
        // With no caller transaction, TTL preparation (direct drainage
        // writes), conversion reads, and the batch apply would each commit
        // on their own, so a conversion error after preparation would leave
        // drained buckets committed without the write. Span all of it with
        // one owned transaction and commit only once the batch applied.
        let caller_transaction = transaction;
        let owned_transaction =
            (apply && transaction.is_none()).then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(caller_transaction);
        if apply {
            self.prepare_drive_operations_time_range_ttl(
                &operations,
                block_info,
                transaction,
                platform_version,
            )?;
        }
        let mut low_level_operations = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        // The document operations of a moderator's deletion, whose removals refund nobody
        let mut forfeited_low_level_operations = vec![];

        let mut finalize_tasks: Vec<DriveOperationFinalizeTask> = Vec::new();

        for drive_op in operations {
            if let Some(tasks) = drive_op.finalization_tasks(platform_version)? {
                finalize_tasks.extend(tasks);
            }

            let forfeited = separates_forfeited_operations
                && matches!(drive_op, DriveOperation::DocumentOperation(_));
            let mut converted = drive_op.into_low_level_drive_operations_after_ttl_drain(
                self,
                &mut estimated_costs_only_with_layer_info,
                block_info,
                transaction,
                platform_version,
            )?;
            if forfeited {
                forfeited_low_level_operations.append(&mut converted);
            } else {
                low_level_operations.append(&mut converted);
            }
        }

        let repaid_identity_debt =
            LowLevelDriveOperation::take_repaid_identity_debt(&mut low_level_operations)?;

        let mut cost_operations = vec![];

        let forfeited_estimated_costs_only_with_layer_info =
            if forfeited_low_level_operations.is_empty() {
                None
            } else {
                Some(estimated_costs_only_with_layer_info.clone())
            };
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            low_level_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;
        if let Some(estimated_costs_only_with_layer_info) =
            forfeited_estimated_costs_only_with_layer_info
        {
            let mut forfeited_cost_operations = vec![];
            self.apply_batch_low_level_drive_operations(
                estimated_costs_only_with_layer_info,
                transaction,
                forfeited_low_level_operations,
                &mut forfeited_cost_operations,
                &platform_version.drive,
            )?;
            forfeit_storage_refunds(&mut forfeited_cost_operations);
            cost_operations.append(&mut forfeited_cost_operations);
        } else if forfeits_storage_refunds {
            // One batch, in which only the document operations free bytes: it forfeits whole.
            forfeit_storage_refunds(&mut cost_operations);
        }
        self.apply_repaid_identity_debt_to_processing_pool(
            repaid_identity_debt,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        // Read the pool after ordinary writes and debt repayment so neither can overwrite
        // the returned allocation. This transfer shares their transaction and is not billed.
        if apply && processing_pool_credits > 0 {
            let pool_operation = self.add_epoch_processing_credits_for_distribution_operation(
                &block_info.epoch,
                processing_pool_credits,
                transaction,
                platform_version,
            )?;
            self.apply_batch_low_level_drive_operations(
                None,
                transaction,
                vec![pool_operation],
                &mut vec![],
                &platform_version.drive,
            )?;
        }
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }

        // Execute drive operation callbacks after updating state. Nothing was written when
        // only estimating, so there is nothing to finalize. The tasks read through the
        // caller's transaction; an owned one was committed just above, and `caller_transaction`
        // is `None` exactly then, so they read committed state.
        if apply {
            for task in finalize_tasks {
                task.execute(self, caller_transaction, platform_version)?;
            }
        }

        Drive::calculate_fee(
            None,
            Some(cost_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )
    }
}

/// Turns every removal attributed to an identity into a removal attributed to nobody: the same
/// bytes leave the system (`FeeResult::removed_bytes_from_system`), and no refund is computed
/// for them. Given the costs of a batch in which only a moderator's deletion's document
/// operations free bytes: the document's, whoever paid for it, its owner or an earlier one, and
/// those of any index subtree the deletion empties, whoever created it.
fn forfeit_storage_refunds(cost_operations: &mut [LowLevelDriveOperation]) {
    for operation in cost_operations.iter_mut() {
        let LowLevelDriveOperation::CalculatedCostOperation(cost) = operation else {
            continue;
        };
        if let StorageRemovedBytes::SectionedStorageRemoval(_) = &cost.storage_cost.removed_bytes {
            let removed_bytes = cost.storage_cost.removed_bytes.total_removed_bytes();
            cost.storage_cost.removed_bytes =
                StorageRemovedBytes::BasicStorageRemoval(removed_bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_POOL_PROCESSING_FEES;
    use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use crate::drive::credit_pools::epochs::paths::EpochProposers;
    use crate::error::drive::DriveError;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::{GroveDbOpBatch, IdentityOperationType};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::test_utils::identities::create_test_identity;
    use dpp::block::epoch::Epoch;
    use grovedb::Element;
    use grovedb_costs::storage_cost::removal::StorageRemovalPerEpochByIdentifier;
    use grovedb_costs::storage_cost::StorageCost;
    use grovedb_costs::OperationCost;
    use intmap::IntMap;

    fn pool_credit(amount: Credits) -> DriveOperation<'static> {
        DriveOperation::SystemOperation(SystemOperationType::AddToCurrentEpochProcessingPool {
            amount,
        })
    }

    fn debt_test_block_info() -> BlockInfo {
        BlockInfo::default_with_epoch(Epoch::new(2).unwrap())
    }

    fn indebted_identity(
        drive: &Drive,
        id: [u8; 32],
        owed: Credits,
        platform_version: &PlatformVersion,
    ) -> [u8; 32] {
        create_test_identity(drive, id, Some(7), None, platform_version).unwrap();
        let operation = drive
            .update_identity_negative_credit_operation(id, owed, platform_version)
            .unwrap();
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                vec![operation],
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();
        id
    }

    fn balance(drive: &Drive, id: [u8; 32], platform_version: &PlatformVersion) -> Credits {
        drive
            .fetch_identity_balance(id, None, platform_version)
            .unwrap()
            .unwrap()
    }

    fn debt(drive: &Drive, id: [u8; 32], platform_version: &PlatformVersion) -> Credits {
        drive
            .fetch_identity_negative_balance_operations(
                id,
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .unwrap()
            .unwrap()
    }

    fn processing_pool(drive: &Drive, platform_version: &PlatformVersion) -> Credits {
        match drive.get_epoch_processing_credits_for_distribution(
            &debt_test_block_info().epoch,
            None,
            platform_version,
        ) {
            Ok(credits) => credits,
            Err(Error::GroveDB(error))
                if matches!(error.as_ref(), grovedb::Error::PathKeyNotFound(_)) =>
            {
                0
            }
            Err(error) => panic!("processing pool read failed: {error}"),
        }
    }

    #[test]
    fn should_apply_processing_pool_credits_without_ordinary_operations_in_the_same_transaction() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let block_info = debt_test_block_info();
        let transaction = drive.grove.start_transaction();
        let fees = drive
            .apply_drive_operations(
                vec![pool_credit(11), pool_credit(13)],
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .unwrap();
        assert_eq!(
            fees,
            FeeResult::default(),
            "internal pool returns are not billed"
        );
        assert_eq!(
            drive
                .get_epoch_processing_credits_for_distribution(
                    &block_info.epoch,
                    Some(&transaction),
                    platform_version
                )
                .unwrap(),
            24
        );
        assert_eq!(
            processing_pool(&drive, platform_version),
            0,
            "caller transaction is not committed"
        );
        drive
            .commit_transaction(transaction, &platform_version.drive)
            .unwrap();
        drive
            .apply_drive_operations(
                vec![pool_credit(7)],
                true,
                &block_info,
                None,
                platform_version,
                None,
            )
            .unwrap();
        assert_eq!(processing_pool(&drive, platform_version), 31);
    }

    #[test]
    fn should_add_processing_pool_returns_after_ordinary_pool_writes_and_debt_repayment() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let block_info = debt_test_block_info();
        let recipient = indebted_identity(&drive, [7; 32], 7, platform_version);
        let pool_write = block_info
            .epoch
            .update_processing_fee_pool_operation(110)
            .unwrap();
        let batch = vec![
            pool_credit(13),
            DriveOperation::IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                identity_id: recipient,
                added_balance: 20,
            }),
            pool_credit(11),
            DriveOperation::GroveDBOpBatch(GroveDbOpBatch::from_operations(vec![pool_write])),
        ];
        drive
            .apply_drive_operations(batch, true, &block_info, None, platform_version, None)
            .unwrap();
        assert_eq!(balance(&drive, recipient, platform_version), 13);
        assert_eq!(debt(&drive, recipient, platform_version), 0);
        assert_eq!(processing_pool(&drive, platform_version), 110 + 7 + 24);
    }

    #[test]
    fn should_estimate_processing_pool_returns_without_reading_or_writing_the_pool() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let block_info = debt_test_block_info();
        let recipient = indebted_identity(&drive, [7; 32], 7, platform_version);
        // An invalid pool would reject an applied return. An estimate must never read it.
        drive
            .grove
            .insert(
                &block_info.epoch.get_path(),
                KEY_POOL_PROCESSING_FEES,
                Element::new_item(vec![1]),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .unwrap();
        let before = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .unwrap();
        let ordinary =
            DriveOperation::IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                identity_id: recipient,
                added_balance: 20,
            });
        let expected = drive
            .apply_drive_operations(
                vec![ordinary.clone()],
                false,
                &block_info,
                None,
                platform_version,
                None,
            )
            .unwrap();
        let actual = drive
            .apply_drive_operations(
                vec![pool_credit(24), ordinary],
                false,
                &block_info,
                None,
                platform_version,
                None,
            )
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            drive
                .apply_drive_operations(
                    vec![pool_credit(24)],
                    false,
                    &block_info,
                    None,
                    platform_version,
                    None
                )
                .unwrap(),
            FeeResult::default()
        );
        assert_eq!(
            drive
                .apply_drive_operations(
                    vec![pool_credit(0)],
                    true,
                    &block_info,
                    None,
                    platform_version,
                    None
                )
                .unwrap(),
            FeeResult::default()
        );
        assert_eq!(
            drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .unwrap(),
            before
        );
        assert_eq!(balance(&drive, recipient, platform_version), 0);
        assert_eq!(debt(&drive, recipient, platform_version), 7);
    }

    #[test]
    fn should_refuse_processing_pool_credit_accumulation_overflow_before_writing() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let recipient = indebted_identity(&drive, [7; 32], 0, platform_version);
        let before = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .unwrap();
        for apply in [false, true] {
            let result = drive.apply_drive_operations(
                vec![
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id: recipient,
                            added_balance: 9,
                        },
                    ),
                    pool_credit(u64::MAX),
                    pool_credit(1),
                ],
                apply,
                &debt_test_block_info(),
                None,
                platform_version,
                None,
            );
            assert!(matches!(result, Err(Error::Fee(FeeError::Overflow(_)))));
            assert_eq!(
                drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .unwrap(),
                before
            );
        }
        assert_eq!(balance(&drive, recipient, platform_version), 0);
    }

    #[test]
    fn should_roll_back_owned_transaction_when_a_post_batch_processing_pool_return_fails() {
        let platform_version = PlatformVersion::latest();
        for corrupted in [true, false] {
            let drive = setup_drive_with_initial_state_structure(None);
            let block_info = debt_test_block_info();
            let recipient = indebted_identity(&drive, [7; 32], 0, platform_version);
            let pool = if corrupted {
                Element::new_item(vec![1])
            } else {
                Element::new_sum_item(i64::MAX)
            };
            drive
                .grove
                .insert(
                    &block_info.epoch.get_path(),
                    KEY_POOL_PROCESSING_FEES,
                    pool,
                    None,
                    None,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .unwrap();
            let before = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .unwrap();
            let batch = vec![
                DriveOperation::IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                    identity_id: recipient,
                    added_balance: 9,
                }),
                pool_credit(1),
            ];
            let result = drive.apply_drive_operations(
                batch.clone(),
                true,
                &block_info,
                None,
                platform_version,
                None,
            );
            if corrupted {
                assert!(matches!(
                    result,
                    Err(Error::Drive(DriveError::UnexpectedElementType(_)))
                ));
            } else {
                assert!(matches!(result, Err(Error::Protocol(_))));
            }
            assert_eq!(
                drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .unwrap(),
                before
            );
            assert_eq!(balance(&drive, recipient, platform_version), 0);
            // A caller-owned transaction retains writes until its caller rolls back/drops it.
            let transaction = drive.grove.start_transaction();
            assert!(drive
                .apply_drive_operations(
                    batch,
                    true,
                    &block_info,
                    Some(&transaction),
                    platform_version,
                    None
                )
                .is_err());
            assert_eq!(
                drive
                    .fetch_identity_balance(recipient, Some(&transaction), platform_version)
                    .unwrap(),
                Some(9)
            );
            drop(transaction);
            assert_eq!(
                drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .unwrap(),
                before
            );
        }
    }

    #[test]
    fn should_refuse_processing_pool_returns_in_legacy_apply_and_generic_conversion() {
        let platform_version = PlatformVersion::latest();
        let legacy = PlatformVersion::get(13).unwrap();
        let drive = setup_drive_with_initial_state_structure(None);
        let block_info = debt_test_block_info();
        for amount in [0, 1] {
            for apply in [false, true] {
                assert!(matches!(
                    drive.apply_drive_operations(
                        vec![pool_credit(amount)],
                        apply,
                        &block_info,
                        None,
                        legacy,
                        None
                    ),
                    Err(Error::Drive(DriveError::NotSupported(_)))
                ));
            }
            for version in [legacy, platform_version] {
                assert!(matches!(
                    drive.convert_drive_operations_to_grove_operations(
                        vec![pool_credit(amount)],
                        &block_info,
                        None,
                        version
                    ),
                    Err(Error::Drive(DriveError::NotSupported(_)))
                ));
            }
        }
        assert_eq!(processing_pool(&drive, platform_version), 0);
    }

    #[test]
    fn should_attribute_a_forfeited_removal_to_nobody() {
        let mut per_epoch = IntMap::new();
        per_epoch.insert(0u16, 40u32);
        per_epoch.insert(3u16, 2u32);
        let mut removal = StorageRemovalPerEpochByIdentifier::default();
        removal.insert([7; 32], per_epoch);

        let mut cost_operations = vec![
            LowLevelDriveOperation::CalculatedCostOperation(OperationCost {
                storage_cost: StorageCost {
                    added_bytes: 5,
                    replaced_bytes: 0,
                    removed_bytes: StorageRemovedBytes::SectionedStorageRemoval(removal),
                },
                ..Default::default()
            }),
            LowLevelDriveOperation::CalculatedCostOperation(OperationCost {
                storage_cost: StorageCost {
                    added_bytes: 0,
                    replaced_bytes: 0,
                    removed_bytes: StorageRemovedBytes::BasicStorageRemoval(9),
                },
                ..Default::default()
            }),
        ];

        forfeit_storage_refunds(&mut cost_operations);

        let removed: Vec<&StorageRemovedBytes> = cost_operations
            .iter()
            .map(|operation| match operation {
                LowLevelDriveOperation::CalculatedCostOperation(cost) => {
                    &cost.storage_cost.removed_bytes
                }
                _ => unreachable!("only calculated costs were given"),
            })
            .collect();
        assert_eq!(
            removed,
            vec![
                &StorageRemovedBytes::BasicStorageRemoval(42),
                &StorageRemovedBytes::BasicStorageRemoval(9),
            ]
        );
    }
}
