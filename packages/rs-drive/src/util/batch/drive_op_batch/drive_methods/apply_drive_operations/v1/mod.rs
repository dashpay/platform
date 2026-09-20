use crate::util::batch::DriveOperation;

use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb_costs::storage_cost::removal::{
    StorageRemovalPerEpochByIdentifier, StorageRemovedBytes,
};
use intmap::IntMap;

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
    /// refund forfeiture ([`DriveOperation::storage_refund_forfeiture`], a moderator's document
    /// deletion) refunds nobody but the identity the forfeiture spares: the bytes it removes still leave the system, but whoever paid
    /// for them gets nothing back, and the credits stay in the storage pools they were
    /// distributed to. An estimate carries no refund to begin with, so `check_tx` sees the
    /// same fee with or without the forfeiture.
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
        if operations.is_empty() {
            return Ok(FeeResult::default());
        }
        let storage_refund_forfeiture = operations
            .iter()
            .find_map(DriveOperation::storage_refund_forfeiture);
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

        let mut cost_operations = vec![];

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            low_level_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }

        if let Some(spared) = storage_refund_forfeiture {
            forfeit_storage_refunds(&mut cost_operations, spared);
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

/// The identity GroveDB's removal sections use for storage nobody owns, which the fee
/// calculation counts as removed from the system and refunds to nobody.
const NOBODY: [u8; 32] = [0; 32];

/// Turns every removal attributed to an identity other than `spared` into a removal attributed
/// to nobody: the same bytes leave the system (`FeeResult::removed_bytes_from_system`), and no
/// refund is computed for them. What is attributed to `spared` stays as it is.
fn forfeit_storage_refunds(
    cost_operations: &mut [LowLevelDriveOperation],
    spared: Option<Identifier>,
) {
    for operation in cost_operations.iter_mut() {
        let LowLevelDriveOperation::CalculatedCostOperation(cost) = operation else {
            continue;
        };
        let StorageRemovedBytes::SectionedStorageRemoval(sections) =
            &mut cost.storage_cost.removed_bytes
        else {
            continue;
        };
        let kept = spared.and_then(|identity| sections.remove_entry(identity.as_bytes()));
        let forfeited: u32 = sections
            .values()
            .flat_map(|per_epoch| per_epoch.values())
            .sum();
        cost.storage_cost.removed_bytes = match kept {
            None => StorageRemovedBytes::BasicStorageRemoval(forfeited),
            Some((identity, per_epoch)) => {
                let mut sections = StorageRemovalPerEpochByIdentifier::default();
                sections.insert(identity, per_epoch);
                if forfeited > 0 {
                    let mut to_nobody = IntMap::new();
                    to_nobody.insert(0u16, forfeited);
                    sections.insert(NOBODY, to_nobody);
                }
                StorageRemovedBytes::SectionedStorageRemoval(sections)
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        forfeit_storage_refunds(&mut cost_operations, None);

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
    fn should_keep_what_is_owed_to_the_spared_identity() {
        let section = |epoch: u16, bytes: u32| {
            let mut per_epoch = IntMap::new();
            per_epoch.insert(epoch, bytes);
            per_epoch
        };
        let mut removal = StorageRemovalPerEpochByIdentifier::default();
        removal.insert([7; 32], section(0, 40));
        removal.insert([8; 32], section(2, 5));
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

        forfeit_storage_refunds(&mut cost_operations, Some(Identifier::from([8; 32])));

        let mut expected = StorageRemovalPerEpochByIdentifier::default();
        expected.insert([8; 32], section(2, 5));
        expected.insert(NOBODY, section(0, 40));
        let LowLevelDriveOperation::CalculatedCostOperation(cost) = &cost_operations[0] else {
            unreachable!("a calculated cost was given");
        };
        assert_eq!(
            cost.storage_cost.removed_bytes,
            StorageRemovedBytes::SectionedStorageRemoval(expected)
        );
    }
}
