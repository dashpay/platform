use crate::util::batch::{DriveOperation, IdentityOperationType};

use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes;

use crate::util::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::Credits;
use std::collections::{BTreeMap, HashMap};

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
    /// Credits the batch adds to an identity that repay its debt
    /// ([`LowLevelDriveOperation::RepaidIdentityDebt`], from `add_to_identity_balance_operations`
    /// 1) go to the processing fee pool of the block's epoch once the batch applied: the debt
    /// stood for processing fees that never reached a pool, and otherwise the credits would
    /// reach no balance the credit sum counts. The pool write reads the state the batch left,
    /// so it adds to a pool write the batch made itself (the fee distribution at the end of a
    /// block) instead of racing it, and it is not billed. An estimate reads no debt and repays
    /// none. Every write the batch makes to one identity's balance is merged into one net write
    /// first ([`merge_identity_balance_writes`]): each is converted against the balance and debt
    /// committed before the batch, so a second write would replace the first, and two credits
    /// to an indebted identity would both repay the same debt.
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
        let operations = merge_identity_balance_writes(operations)?;
        if operations.is_empty() {
            return Ok(FeeResult::default());
        }
        let forfeits_storage_refunds = operations
            .iter()
            .any(DriveOperation::forfeits_storage_refunds);
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
            forfeit_storage_refunds(&mut cost_operations);
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

/// Merges every `AddToIdentityBalance` and `RemoveFromIdentityBalance` of one identity in
/// `operations` into one net write, in the place of the first, or none when they cancel out. A
/// batch that writes each identity's balance at most once is returned as it is. Each write is
/// converted against the balance and debt committed before the batch, and a batch keeps only
/// the last write of a key, so unmerged, a second write to one identity would replace the
/// first, and two credits to an indebted one would each report repaying the same debt. The
/// merged write applies them as if in turn: a debt exists only at a zero balance, so writes
/// that would succeed one after the other net to a credit that repays it once.
fn merge_identity_balance_writes(
    operations: Vec<DriveOperation<'_>>,
) -> Result<Vec<DriveOperation<'_>>, Error> {
    // Most batches write at most one balance: nothing to merge, and no map to build
    if operations
        .iter()
        .filter(|operation| identity_balance_write(operation).is_some())
        .nth(1)
        .is_none()
    {
        return Ok(operations);
    }
    let mut writes: BTreeMap<[u8; 32], (usize, i128)> = BTreeMap::new();
    for (identity_id, change) in operations.iter().filter_map(identity_balance_write) {
        let (count, net) = writes.entry(identity_id).or_default();
        *count += 1;
        *net = net
            .checked_add(change)
            .ok_or(Error::Fee(FeeError::Overflow(
                "the balance writes one batch makes to an identity overflow",
            )))?;
    }
    if writes.values().all(|(count, _)| *count == 1) {
        return Ok(operations);
    }
    let mut merged = Vec::with_capacity(operations.len());
    for operation in operations {
        let Some((identity_id, _)) = identity_balance_write(&operation) else {
            merged.push(operation);
            continue;
        };
        match writes.get_mut(&identity_id) {
            Some((1, _)) => merged.push(operation),
            // The first write of an identity written more than once: the net one goes here,
            // and the entry is marked done so the later ones are dropped
            Some((count, net)) if *count > 1 => {
                merged.extend(net_identity_balance_write(identity_id, *net)?);
                *count = 0;
            }
            _ => {}
        }
    }
    Ok(merged)
}

/// The identity whose balance `operation` writes and the signed change it makes, if it is a
/// credit or a debit of an identity balance
fn identity_balance_write(operation: &DriveOperation) -> Option<([u8; 32], i128)> {
    match operation {
        DriveOperation::IdentityOperation(IdentityOperationType::AddToIdentityBalance {
            identity_id,
            added_balance,
        }) => Some((*identity_id, i128::from(*added_balance))),
        DriveOperation::IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
            identity_id,
            balance_to_remove,
        }) => Some((*identity_id, -i128::from(*balance_to_remove))),
        _ => None,
    }
}

/// The one write that makes the signed change `net` to `identity_id`'s balance, or none when
/// it is zero
fn net_identity_balance_write<'a>(
    identity_id: [u8; 32],
    net: i128,
) -> Result<Option<DriveOperation<'a>>, Error> {
    if net == 0 {
        return Ok(None);
    }
    let amount = Credits::try_from(net.unsigned_abs()).map_err(|_| {
        Error::Fee(FeeError::Overflow(
            "the merged writes of one identity balance overflow credits",
        ))
    })?;
    Ok(Some(DriveOperation::IdentityOperation(if net > 0 {
        IdentityOperationType::AddToIdentityBalance {
            identity_id,
            added_balance: amount,
        }
    } else {
        IdentityOperationType::RemoveFromIdentityBalance {
            identity_id,
            balance_to_remove: amount,
        }
    })))
}

/// Turns every removal attributed to an identity into a removal attributed to nobody: the same
/// bytes leave the system (`FeeResult::removed_bytes_from_system`), and no refund is computed
/// for them. The whole batch, which is exact: a moderator's deletion removes the document and
/// nothing else, since its removal record is written once (a document id is produced at most
/// once) and the nonce it bumps keeps its size.
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
    use grovedb_costs::storage_cost::removal::StorageRemovalPerEpochByIdentifier;
    use grovedb_costs::storage_cost::StorageCost;
    use grovedb_costs::OperationCost;
    use intmap::IntMap;

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
