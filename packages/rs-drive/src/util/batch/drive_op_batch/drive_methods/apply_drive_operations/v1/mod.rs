use crate::util::batch::DriveOperation;

use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb_costs::storage_cost::removal::{StorageRemovedBytes, UNKNOWN_EPOCH};
use intmap::IntMap;

use crate::util::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use std::collections::{BTreeSet, HashMap};

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
    /// deletion) refunds nobody: the bytes it removes still leave the system, but whoever paid
    /// for them gets nothing back, and the credits stay in the storage pools they were
    /// distributed to. An estimate carries no refund to begin with, so `check_tx` sees the
    /// same fee with or without the forfeiture.
    ///
    /// Every write of one identity balance, one contract fee pot or one prefunded specialized
    /// balance is also merged into one ([`DriveOperation::merge_balance_writes`]): each
    /// computes the new value from the one committed before the batch, so a second write in the
    /// same batch would replace the first and the credits would no longer add up. Token writes
    /// cannot be merged the same way, so a batch that writes one token balance or token supply
    /// twice is refused ([`DriveOperation::refuse_repeated_token_balance_writes`]); no state
    /// transition makes one. A batch that writes each key once is applied exactly as by
    /// generation 0.
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
        let operations = DriveOperation::merge_balance_writes(operations)?;
        if operations.is_empty() {
            return Ok(FeeResult::default());
        }
        let forfeits_storage_refunds = operations
            .iter()
            .any(DriveOperation::forfeits_storage_refunds);
        let spared_storage_refunds: BTreeSet<[u8; 32]> = operations
            .iter()
            .filter_map(DriveOperation::spared_storage_refund)
            .map(|identity| identity.to_buffer())
            .collect();
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

        let mut finalize_tasks: Vec<DriveOperationFinalizeTask> = Vec::new();

        for drive_op in operations {
            if let Some(tasks) = drive_op.finalization_tasks(platform_version)? {
                finalize_tasks.extend(tasks);
            }

            low_level_operations.append(
                &mut drive_op.into_low_level_drive_operations_after_ttl_drain(
                    self,
                    &mut estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )?,
            );
        }

        let repaid_identity_debt =
            LowLevelDriveOperation::take_repaid_identity_debt(&mut low_level_operations)?;

        let mut cost_operations = vec![];

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            low_level_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;
        self.apply_repaid_identity_debt_to_processing_pool(
            repaid_identity_debt,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }

        if forfeits_storage_refunds {
            forfeit_storage_refunds(&mut cost_operations, &spared_storage_refunds);
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

/// Turns every removal attributed to an identity other than the `spared` ones into a removal
/// attributed to nobody: the same bytes leave the system (`FeeResult::removed_bytes_from_system`),
/// and no refund is computed for them. A moderator's deletion removes the document, the nonce it
/// bumps keeps its size, and a fresh removal record frees nothing; when it replaces the record
/// of a deletion a moderator restored, the bytes the fresh record gives up are the record
/// holder's, and the deletion spares that identity, refunded as for any replacement. With no
/// spared identity among its removals, an operation's removal becomes a basic one, as before
/// any identity was spared.
fn forfeit_storage_refunds(
    cost_operations: &mut [LowLevelDriveOperation],
    spared: &BTreeSet<[u8; 32]>,
) {
    for operation in cost_operations.iter_mut() {
        let LowLevelDriveOperation::CalculatedCostOperation(cost) = operation else {
            continue;
        };
        let StorageRemovedBytes::SectionedStorageRemoval(removals) =
            &mut cost.storage_cost.removed_bytes
        else {
            continue;
        };
        if !removals.keys().any(|identity| spared.contains(identity)) {
            let removed_bytes = cost.storage_cost.removed_bytes.total_removed_bytes();
            cost.storage_cost.removed_bytes =
                StorageRemovedBytes::BasicStorageRemoval(removed_bytes);
            continue;
        }
        // The system identifier's removals are summed into the bytes removed from the system
        // whatever their epoch, and refund nobody
        let system = [0u8; 32];
        let forfeited_identities: Vec<[u8; 32]> = removals
            .keys()
            .filter(|identity| !spared.contains(*identity) && **identity != system)
            .copied()
            .collect();
        let mut forfeited_bytes = 0u32;
        for identity in forfeited_identities {
            if let Some(by_epoch) = removals.remove(&identity) {
                forfeited_bytes = by_epoch
                    .values()
                    .fold(forfeited_bytes, |sum, bytes| sum.saturating_add(*bytes));
            }
        }
        if forfeited_bytes > 0 {
            let system_removals = removals.entry(system).or_insert_with(IntMap::new);
            match system_removals.get_mut(UNKNOWN_EPOCH) {
                Some(bytes) => *bytes = bytes.saturating_add(forfeited_bytes),
                None => {
                    system_removals.insert(UNKNOWN_EPOCH, forfeited_bytes);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb_costs::storage_cost::removal::StorageRemovalPerEpochByIdentifier;
    use grovedb_costs::storage_cost::StorageCost;
    use grovedb_costs::OperationCost;

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

        forfeit_storage_refunds(&mut cost_operations, &BTreeSet::new());

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

    #[test]
    fn should_keep_the_refund_of_a_spared_identity() {
        let per_epoch = |entries: &[(u16, u32)]| {
            let mut per_epoch = IntMap::new();
            for (epoch, bytes) in entries {
                per_epoch.insert(*epoch, *bytes);
            }
            per_epoch
        };
        let mut removal = StorageRemovalPerEpochByIdentifier::default();
        // The deleted document's owner, the moderator holding the replaced record, and bytes
        // that were nobody's already
        removal.insert([7; 32], per_epoch(&[(0, 40), (3, 2)]));
        removal.insert([8; 32], per_epoch(&[(1, 30)]));
        removal.insert([0; 32], per_epoch(&[(2, 5)]));
        let mut cost_operations = vec![LowLevelDriveOperation::CalculatedCostOperation(
            OperationCost {
                storage_cost: StorageCost {
                    added_bytes: 0,
                    replaced_bytes: 0,
                    removed_bytes: StorageRemovedBytes::SectionedStorageRemoval(removal),
                },
                ..Default::default()
            },
        )];

        forfeit_storage_refunds(&mut cost_operations, &BTreeSet::from([[8; 32]]));

        let LowLevelDriveOperation::CalculatedCostOperation(cost) = &cost_operations[0] else {
            unreachable!("only calculated costs were given");
        };
        let StorageRemovedBytes::SectionedStorageRemoval(removals) =
            &cost.storage_cost.removed_bytes
        else {
            panic!("the spared identity keeps a sectioned removal");
        };
        // The spared identity keeps its removal, per epoch; the owner's joins the system's
        assert_eq!(removals.len(), 2);
        assert_eq!(removals[&[8; 32]].get(1), Some(&30));
        assert_eq!(removals[&[0; 32]].values().sum::<u32>(), 5 + 42);
        assert_eq!(cost.storage_cost.removed_bytes.total_removed_bytes(), 77);
    }
}
