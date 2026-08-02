use crate::drive::document::estimation_costs::estimated_sum_trees_for_value_tree_type::estimated_sum_trees_for_value_tree_type;
use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::DriveKeyInfo::KeyRef;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::document_type::IndexLevel;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds indices for an index level and recurses.
    ///
    /// `parent_value_tree_type` carries the **exact `TreeType`** of the
    /// value tree at `index_path_info`. The choice of wrapper for
    /// continuation property-name trees stored as children of the
    /// value tree depends on which axes (count, sum, or both) the
    /// parent aggregates:
    /// - `NormalTree` parent → no wrapping needed.
    /// - `CountTree` / `ProvableCountTree` → `Element::NonCounted`.
    /// - `SumTree` / `ProvableSumTree` / `BigSumTree` → `Element::NotSummed`.
    /// - `CountSumTree` / `ProvableCountSumTree` /
    ///   `ProvableCountProvableSumTree` → `Element::NotCountedOrSummed`.
    ///
    /// In every aggregating case the wrapped continuation contributes
    /// 0 to the parent's per-axis aggregates, so the value tree's
    /// `count_value` / `sum_value` / (count + sum) equals exactly the
    /// contribution of the `[0]` ref-bucket and not the structural
    /// overhead of compound continuations.
    ///
    /// Pre-v3 contracts only ever produce `NormalTree` / `CountTree` /
    /// `ProvableCountTree` parents (the sum flags default to
    /// `false`/`None`), so the extended logic is a bit-identical no-op
    /// for them — the new sum-side wrapper arms stay dormant and the
    /// produced grovedb layout matches what v0 (count-only) used to
    /// emit.
    ///
    /// ## Why "countable" gates the value-tree type, not "range_countable"
    ///
    /// The value tree's purpose is to carry a per-value doc count for fast
    /// point-lookup count proofs (no need to descend one more layer to a
    /// `[0]`-child CountTree). That benefit applies to **every** countable
    /// terminator — `range_countable: true` is only needed to *also* upgrade
    /// the property-name tree to `ProvableCountTree` for
    /// `AggregateCountOnRange` queries. Gating the value tree on
    /// `countable.is_countable()` rather than `range_countable` lets
    /// plain-countable indexes (e.g. `byBrand`) emit the same compact
    /// point-lookup proof shape as rangeCountable ones, without paying the
    /// `ProvableCountTree` cost at the property-name level. The exact same
    /// reasoning applies on the sum side: `summable.is_some()` gates the
    /// value-tree type (NormalTree → SumTree), `range_summable: true`
    /// additionally upgrades the property-name level (NormalTree →
    /// ProvableSumTree) so `AggregateSumOnRange` queries land.
    ///
    /// When both flag families are set the choices compose into the
    /// combined variants (`CountSumTree` / `ProvableCountSumTree`) via
    /// the same dispatch table documented on
    /// [`crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType::primary_key_tree_type`]'s
    /// v1 arm.
    ///
    /// Continuation wrapping under the new rule: when the parent value tree
    /// aggregates anything (count, sum, or both), every child continuation
    /// property-name tree gets `Element::NonCounted`-wrapped so the
    /// parent's aggregate equals exactly the contribution from the `[0]`
    /// ref-bucket. Without the wrap, each continuation would contribute
    /// its own aggregate (a single doc for `NormalTree`, a sub-count for
    /// `ProvableCountTree`, a sub-sum for `ProvableSumTree`, etc.) and
    /// the parent would over-aggregate.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_indices_for_index_level_for_contract_operations_v1(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_level: &IndexLevel,
        mut any_fields_null: bool,
        mut all_fields_null: bool,
        parent_value_tree_type: TreeType,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        event_id: [u8; 32],
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(index_type) = index_level.has_index_with_type() {
            self.add_reference_for_index_level_for_contract_operations(
                document_and_contract_info,
                index_path_info.clone(),
                index_type,
                any_fields_null,
                all_fields_null,
                previous_batch_operations,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        let document_type = document_and_contract_info.document_type;

        let sub_level_index_count = index_level.sub_levels().len() as u32;

        // The current level (the value tree at index_path_info) has
        // exactly the TreeType the caller already computed — pass it
        // through so the layer info, the recursive call, and the
        // wrapper-choice for child continuations all agree on the
        // exact variant (no lossy bool → CountTree projection).
        let current_layer_tree_type = parent_value_tree_type;
        // True iff the parent value tree aggregates anything (count,
        // sum, or both). Matches the legacy bool semantic — used below
        // to decide whether to NonCounted/NotSummed/NotCountedOrSummed-
        // wrap continuation children. Non-aggregating parents emit
        // plain empty trees via the unwrapped helper.
        let parent_value_tree_aggregates = !matches!(parent_value_tree_type, TreeType::NormalTree);

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
                EstimatedLayerInformation {
                    tree_type: current_layer_tree_type,
                    estimated_layer_count: ApproximateElements(sub_level_index_count + 1),
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        // fourth we need to store a reference to the document for each index
        for (name, sub_level) in index_level.sub_levels() {
            // Two separate flags, deliberately kept distinct:
            //
            // - `sub_level_is_countable_terminator`: the sub_level has an
            //   index AND that index is countable (any tier). Drives the
            //   value-tree type and the NonCounted wrapping decision.
            //   Pure prefix levels (no index at this sub_level) leave this
            //   `false` so their value trees stay `NormalTree` — there's
            //   nothing to count at a prefix-only level.
            // - `sub_level_range_countable`: a stronger flag — the sub_level
            //   is countable AND opts into range-aggregate support. Drives
            //   the property-name tree's upgrade from `NormalTree` to
            //   `ProvableCountTree` (the type `AggregateCountOnRange` walks
            //   over). Implied by `sub_level_is_countable_terminator` per
            //   `Index::range_countable`'s docstring: `range_countable: true`
            //   requires `countable: Countable | CountableAllowingOffset`.
            let sub_level_index_info = sub_level.has_index_with_type();
            let sub_level_is_countable_terminator = sub_level_index_info
                .map(|info| info.countable.is_countable())
                .unwrap_or(false);
            let sub_level_range_countable = sub_level_index_info
                .map(|info| info.range_countable)
                .unwrap_or(false);
            // v3 sum-tree flags (default false/None on contracts written
            // before the sum-tree feature; bit-identical no-op for them).
            let sub_level_is_summable_terminator = sub_level_index_info
                .map(|info| info.summable.is_some())
                .unwrap_or(false);
            let sub_level_range_summable = sub_level_index_info
                .map(|info| info.range_summable)
                .unwrap_or(false);

            // Compose count and sum flags into the right TreeType for
            // the property-name level (the level *above* the value
            // trees, whose keys are this property's distinct values).
            //
            // The four upgrade paths:
            // - `range_countable: true` → ProvableCountTree (existing)
            // - `range_summable: true` → ProvableSumTree (new in v3)
            // - both → ProvableCountSumTree (combined feature, grovedb
            //   PR 670)
            // - neither → NormalTree (default; matches v0 behavior)
            //
            // Plain-countable / plain-summable terminators don't upgrade
            // the property-name level — only their value-tree level (see
            // below). The range-* variants are what `AggregateCountOnRange`
            // / `AggregateSumOnRange` walk over.
            let property_name_tree_type =
                match (sub_level_range_countable, sub_level_range_summable) {
                    (true, true) => TreeType::ProvableCountProvableSumTree,
                    (true, false) => TreeType::ProvableCountTree,
                    (false, true) => TreeType::ProvableSumTree,
                    (false, false) => TreeType::NormalTree,
                };

            // The value tree (one per distinct property value, hosting the
            // `[0]` reference subtree + sibling continuations) becomes an
            // aggregating tree at any countable / summable terminator.
            // This shortens the point-lookup proof by one merk layer per
            // resolved branch — the `[0]` child doesn't need to be
            // descended; the value tree's own `count_value_or_default()`
            // / `sum_value_or_default()` IS the per-branch aggregate,
            // with sibling continuations wrapped `NonCounted` to keep
            // it honest (see `wrap_property_name_tree_non_counted`
            // below).
            //
            // For non-terminator (pure prefix) levels — e.g. `recipient`
            // in a tip-jar contract that has a standalone `[recipient]`
            // index AND a deeper `[recipient, sentAt]` — the standalone
            // index's terminator IS this level so the flags are
            // non-zero; for a contract that has only `[recipient,
            // sentAt]` and no standalone `[recipient]`, the `recipient`
            // level has no `has_index_with_type` and the value tree
            // stays `NormalTree`. Pure prefix walks just descend.
            //
            // Combined feature: `documentsCountable + documentsSummable`
            // at the doctype produces `CountSumTree` at the primary key
            // level (see primary_key_tree_type's v1); the same
            // composition logic applied here at the per-value level
            // produces `CountSumTree` / `ProvableCountSumTree` at the
            // value tree depending on whether the index opts into the
            // range variants too.
            let value_tree_type = match (
                sub_level_is_countable_terminator,
                sub_level_range_countable,
                sub_level_is_summable_terminator,
                sub_level_range_summable,
            ) {
                // Combined count+sum surfaces.
                (true, true, true, true) => TreeType::ProvableCountProvableSumTree,
                (true, false, true, false) => TreeType::CountSumTree,
                (true, true, true, false) => TreeType::ProvableCountSumTree,
                (true, false, true, true) => TreeType::ProvableCountProvableSumTree,
                // Pure count surfaces.
                (true, _, false, false) => TreeType::CountTree,
                // Pure sum surfaces.
                (false, false, true, _) => TreeType::SumTree,
                // Pure NormalTree (no terminator at this level OR no
                // aggregating index opts in).
                (false, _, false, _) => TreeType::NormalTree,
                // Catch-all: range_countable without countable, or
                // range_summable without summable — caught earlier by
                // the DPP validator, but be defensive.
                _ => TreeType::NormalTree,
            };

            // Wrap the property-name tree iff its immediate parent
            // (the value tree at `index_path_info`) aggregates count,
            // sum, or both. The wrapper variant is keyed on
            // `parent_value_tree_type` — the new helper
            // `wrap_in_non_aggregated_for_parent_tree_type`
            // picks NonCounted / NotSummed / NotCountedOrSummed
            // based on what axes the parent aggregates.
            let wrap_property_name_tree_under_aggregating_parent = parent_value_tree_aggregates;

            let property_name_apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: current_layer_tree_type,
                    tree_type: property_name_tree_type,
                    flags_len: storage_flags
                        .map(|s| s.serialized_size())
                        .unwrap_or_default(),
                }
            };

            let value_apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: property_name_tree_type,
                    tree_type: value_tree_type,
                    flags_len: storage_flags
                        .map(|s| s.serialized_size())
                        .unwrap_or_default(),
                }
            };

            let mut sub_level_index_path_info = index_path_info.clone();
            let index_property_key = KeyRef(name.as_bytes());

            let document_index_field = document_and_contract_info
                .owned_document_info
                .document_info
                .get_raw_for_document_type(
                    name,
                    document_type,
                    document_and_contract_info.owned_document_info.owner_id,
                    Some((sub_level, event_id)),
                    platform_version,
                )?
                .unwrap_or_default();

            let path_key_info = index_property_key
                .clone()
                .add_path_info(sub_level_index_path_info.clone());

            // here we are inserting an empty tree that will have a subtree of all other index properties
            if wrap_property_name_tree_under_aggregating_parent {
                self.batch_insert_empty_tree_under_aggregating_parent_if_not_exists(
                    path_key_info.clone(),
                    parent_value_tree_type,
                    property_name_tree_type,
                    *storage_flags,
                    property_name_apply_type,
                    transaction,
                    previous_batch_operations,
                    batch_operations,
                    &platform_version.drive,
                )?;
            } else {
                self.batch_insert_empty_tree_if_not_exists(
                    path_key_info.clone(),
                    property_name_tree_type,
                    *storage_flags,
                    property_name_apply_type,
                    transaction,
                    previous_batch_operations,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }

            sub_level_index_path_info.push(index_property_key)?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type, platform_version)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document top field is too big for being an index on delete",
                    )));
                }

                // The property-name layer's children are value trees,
                // each of type `value_tree_type` derived above from the
                // sub-level's summable / range_summable / countable /
                // range_countable flags. v0 emitted `NoSumTrees` here
                // unconditionally — correct for pre-v12 (when value_tree_type
                // is always NormalTree because the sum/range flags didn't
                // exist), but under-charges v12+ writes on summable /
                // rangeSummable indexes whose value trees carry per-node
                // sum or count contributions. v1 maps `value_tree_type`
                // to the matching `SomeSumTrees` weight slot so the
                // average-case cost includes those bytes.
                estimated_costs_only_with_layer_info.insert(
                    sub_level_index_path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        tree_type: property_name_tree_type,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            document_top_field_estimated_size as u8,
                            estimated_sum_trees_for_value_tree_type(value_tree_type),
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
            // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

            let path_key_info = document_index_field
                .clone()
                .add_path_info(sub_level_index_path_info.clone());

            // here we are inserting the value tree
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info.clone(),
                value_tree_type,
                *storage_flags,
                value_apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                &platform_version.drive,
            )?;

            any_fields_null |= document_index_field.is_empty();
            all_fields_null &= document_index_field.is_empty();

            // we push the actual value of the index path
            sub_level_index_path_info.push(document_index_field)?;
            // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
            // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            // Propagate the actual `value_tree_type` forward — it tracks
            // the exact TreeType of the value tree we just wrote (the
            // one the sub-level will recurse INTO). The next level
            // reads it to pick the correct wrapper variant
            // (NonCounted / NotSummed / NotCountedOrSummed) for its
            // own continuation children.
            self.add_indices_for_index_level_for_contract_operations_v1(
                document_and_contract_info,
                sub_level_index_path_info,
                sub_level,
                any_fields_null,
                all_fields_null,
                value_tree_type,
                previous_batch_operations,
                storage_flags,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            )?;
        }
        Ok(())
    }
}
