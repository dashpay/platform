mod v0;

use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;
use grovedb::element::IndexAxis;
use grovedb::{TransactionArg, TreeType};

/// How the underlying v0 body builds the empty-tree operation.
///
/// Orthogonal to the `ranked_axes` argument every entry point also
/// passes: the mode picks *how the element is wrapped*, the axes decide
/// *which element variant is built*. Only `NotWrapped` admits a
/// non-empty axis list — an indexed tree structurally cannot be wrapped
/// (see `INDEXED_INNER_UNWRAPPABLE` in `fees::op`) — and the v0 body
/// fails closed on the other combinations rather than dropping the axes.
#[derive(Clone, Copy)]
enum EmptyTreeInsertMode {
    /// Plain empty tree of the requested type (non-aggregating parent).
    NotWrapped,
    /// v0 wrapper dispatch keyed on the aggregating parent's tree type —
    /// the diagonal-only matrix
    /// ([`LowLevelDriveOperation::wrap_in_non_aggregated_for_parent_tree_type`]),
    /// consensus-frozen for the pre-v14 index walkers.
    NonAggregatedForParent(TreeType),
    /// v2 zero-contribution dispatch keyed on the aggregating parent's
    /// tree type — the full parent×inner matrix
    /// ([`LowLevelDriveOperation::for_known_path_key_empty_tree_contributing_zero_to_parent`]),
    /// which may emit an unwrapped op when the child contributes zero
    /// naturally. Reachable only from the v2 index walkers.
    ContributingZeroToParent(TreeType),
}

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
                EmptyTreeInsertMode::NotWrapped, // non-aggregating insert, no wrap
                &[],                             // ranked_axes — plain (non-indexed) tree
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

    /// Ranking-aware form of [`Self::batch_insert_empty_tree_if_not_exists`]
    /// for an index's **terminal property-name tree**.
    ///
    /// `ranked_axes` is empty for every index that declares no ranking flag,
    /// in which case this is bit-identical to the plain helper — which is why
    /// the index walkers can call this unconditionally without changing what
    /// they emit for pre-PV14 contracts. When non-empty, `tree_type` is one of
    /// the three indexed variants (grovedb PR 657) and the axes supply the TLV
    /// that `TreeType` alone cannot carry.
    ///
    /// Shares the `batch_insert_empty_tree_if_not_exists` version slot: the
    /// existence check, the pending-operation scan and the delete-cancellation
    /// behaviour are the same code path, and only the element being built
    /// differs.
    #[allow(clippy::too_many_arguments)]
    pub fn batch_insert_empty_index_tree_if_not_exists<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        tree_type: TreeType,
        ranked_axes: &[IndexAxis],
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
                // An indexed tree can never be wrapped — see
                // INDEXED_INNER_UNWRAPPABLE.
                EmptyTreeInsertMode::NotWrapped,
                ranked_axes,
                storage_flags,
                apply_type,
                transaction,
                check_existing_operations,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_index_tree_if_not_exists".to_string(),
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
                EmptyTreeInsertMode::NonAggregatedForParent(aggregating_parent_tree_type),
                &[], // ranked_axes — a wrapped child is never an indexed tree
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

    /// Pushes an "insert empty `tree_type` contributing zero to every axis
    /// its aggregating parent tracks" operation to `drive_operations`, but
    /// only if the path/key doesn't already exist (in current state OR in
    /// pending operations).
    ///
    /// The v2 index walkers' replacement for
    /// [`Self::batch_insert_empty_tree_under_aggregating_parent_if_not_exists`]:
    /// where that helper's v0 wrapper dispatch covers only the diagonal of
    /// the parent×inner matrix (and errors on shared-prefix aggregate
    /// layouts like a summable `[a]` next to a plain compound `[a, b]`),
    /// this one completes the matrix — see
    /// [`LowLevelDriveOperation::for_known_path_key_empty_tree_contributing_zero_to_parent`]
    /// for the full dispatch (including the unwrapped fallback for children
    /// that contribute zero naturally, e.g. a plain continuation under a
    /// sum-only value tree).
    ///
    /// The platform gate is carried by its only callers — the v14+ v2
    /// index walkers and v1 update walker — so pre-v14 behavior can't
    /// reach this path. Crate-private for the same reason: exposing it
    /// would let downstream crates bypass that gate. The grove feature
    /// version is still dispatched like the sibling helpers so a future
    /// v1 of the batch-dedup semantics can't silently diverge here.
    ///
    /// `ranked_axes` must be empty: a ranked index's terminal
    /// property-name tree is an indexed tree, which cannot be wrapped
    /// *and* cannot be silently inserted unwrapped here (the
    /// zero-contribution dispatcher's unwrapped fallback for sum-only
    /// parents would otherwise accept one). Callers pass the resolved
    /// axes rather than `&[]` so that shape fails closed with the
    /// dedicated message instead of losing its secondaries — the same
    /// backstop the v1 walkers got from
    /// [`LowLevelDriveOperation::wrap_in_non_aggregated_for_parent_tree_type`].
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn batch_insert_empty_tree_contributing_zero_to_aggregating_parent_if_not_exists<
        const N: usize,
    >(
        &self,
        path_key_info: PathKeyInfo<N>,
        aggregating_parent_tree_type: TreeType,
        tree_type: TreeType,
        ranked_axes: &[IndexAxis],
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
                EmptyTreeInsertMode::ContributingZeroToParent(aggregating_parent_tree_type),
                ranked_axes,
                storage_flags,
                apply_type,
                transaction,
                check_existing_operations,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "batch_insert_empty_tree_contributing_zero_to_aggregating_parent_if_not_exists"
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
