mod v0;

use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations
    /// Returns true if we inserted
    #[allow(clippy::too_many_arguments)]
    pub fn batch_insert_empty_tree_if_not_exists<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_insert_empty_tree_if_not_exists
        {
            0 => self.batch_insert_empty_tree_if_not_exists_v0(
                path_key_info,
                tree_type,
                None, // wrap_in_non_aggregated_for_parent_tree_type — non-aggregating insert, no wrap
                storage_flags,
                apply_type,
                transaction,
                check_existing_operations,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_tree_if_not_exists".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Pushes an "insert empty `tree_type` wrapped in the appropriate
    /// `Element::NonCounted` / `Element::NotSummed` / `Element::NotCountedOrSummed`
    /// wrapper" operation to `drive_operations`, but only if the path/key
    /// doesn't already exist (in current state OR in pending operations).
    ///
    /// Used by the index walker for sibling continuations that live inside
    /// an aggregating value tree. The wrapper variant is picked based on
    /// the parent's `aggregating_parent_tree_type`:
    /// - count-only parents (CountTree / ProvableCountTree) → `NonCounted`
    /// - sum-only parents (SumTree / ProvableSumTree / BigSumTree) → `NotSummed`
    /// - combined count+sum parents (CountSumTree / ProvableCountSumTree /
    ///   ProvableCountProvableSumTree) → `NotCountedOrSummed`
    ///
    /// Without the wrapper, an empty child tree would contribute 1 to the
    /// parent's `count_value_or_default()` and/or its own aggregate would
    /// be added to the parent's sum; the wrapper makes it contribute 0 on
    /// each suppressed axis so the value tree's aggregates cleanly reflect
    /// "documents at this value" rather than "documents + sibling-
    /// continuation-trees". `tree_type` is left general so nested-
    /// `range_countable`/`range_summable` shapes can pass through any
    /// aggregating continuation variant.
    #[allow(clippy::too_many_arguments)]
    pub fn batch_insert_empty_tree_under_aggregating_parent_if_not_exists<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        aggregating_parent_tree_type: TreeType,
        tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_insert_empty_tree_if_not_exists
        {
            0 => self.batch_insert_empty_tree_if_not_exists_v0(
                path_key_info,
                tree_type,
                Some(aggregating_parent_tree_type),
                storage_flags,
                apply_type,
                transaction,
                check_existing_operations,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_tree_under_aggregating_parent_if_not_exists"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Count-only specialization of
    /// [`Self::batch_insert_empty_tree_under_aggregating_parent_if_not_exists`]
    /// preserved for [`Drive::add_indices_for_index_level_for_contract_operations_v0`]'s
    /// exclusive use. v0 only ever encounters `CountTree` parents
    /// (the v3 sum-tree feature lights up under v1 only), so this
    /// shim hard-codes the parent tree type and forwards to the
    /// general helper. Kept as a separate function so v0's source
    /// text stays bit-identical to its v11-ship state.
    #[allow(clippy::too_many_arguments)]
    pub fn batch_insert_empty_non_counted_tree_if_not_exists<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        self.batch_insert_empty_tree_under_aggregating_parent_if_not_exists(
            path_key_info,
            TreeType::CountTree,
            tree_type,
            storage_flags,
            apply_type,
            transaction,
            check_existing_operations,
            drive_operations,
            drive_version,
        )
    }
}
