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
    /// `parent_value_tree_is_count_tree` reflects whether the value tree at
    /// `index_path_info` is a `CountTree` (because the `IndexLevel` that
    /// produced it is a countable terminator — i.e. `index.countable` is
    /// `Countable` or `CountableAllowingOffset`). When true, every
    /// continuation property-name tree we insert here as a child of that
    /// `CountTree` is wrapped with `Element::NonCounted` so its storage
    /// stays addressable but it contributes 0 to the parent count's
    /// aggregate. Without this, compound continuations would each add 1 (a
    /// `NormalTree` child) — or worse, their own count_value (a
    /// `ProvableCountTree` child in nested-range_countable layouts) — and
    /// double-count documents.
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
    /// `ProvableCountTree` cost at the property-name level.
    ///
    /// Continuation wrapping under the new rule: when the parent value tree
    /// is a `CountTree` (now true for every countable terminator, not just
    /// rangeCountable), every child continuation property-name tree gets
    /// `Element::NonCounted`-wrapped so the parent's count_value equals
    /// exactly the doc count from the `[0]` ref-bucket. Without the wrap,
    /// each continuation would contribute its own `count_value_or_default`
    /// (1 for `NormalTree`, > 0 for `ProvableCountTree`) and the parent
    /// would over-count.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_indices_for_index_level_for_contract_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_level: &IndexLevel,
        mut any_fields_null: bool,
        mut all_fields_null: bool,
        parent_value_tree_is_count_tree: bool,
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
                platform_version,
            )?;
        }

        let document_type = document_and_contract_info.document_type;

        let sub_level_index_count = index_level.sub_levels().len() as u32;

        // The current level (the value tree at index_path_info) is a CountTree
        // when `parent_value_tree_is_count_tree`; otherwise NormalTree.
        // This shows up in the layer info for the layer we're walking through.
        let current_layer_tree_type = if parent_value_tree_is_count_tree {
            TreeType::CountTree
        } else {
            TreeType::NormalTree
        };

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

            // The property-name tree below the current value tree. If the
            // index sub_level is a range_countable terminator we need a
            // `ProvableCountTree` so range queries over the property's
            // distinct values can use grovedb's `AggregateCountOnRange`.
            // Plain countable terminators keep `NormalTree` — they don't
            // need the per-node count aggregation for range support.
            let property_name_tree_type = if sub_level_range_countable {
                TreeType::ProvableCountTree
            } else {
                TreeType::NormalTree
            };

            // The value tree (one per distinct property value, hosting the
            // `[0]` reference subtree + sibling continuations) becomes a
            // `CountTree` at any countable terminator — not just
            // `range_countable` ones. This shortens the point-lookup count
            // proof by one merk layer per resolved branch (the `[0]` child
            // doesn't need to be descended; the value tree's own
            // `count_value_or_default()` IS the per-branch doc count, with
            // sibling continuations wrapped `NonCounted` to keep the count
            // honest — see `wrap_property_name_tree_non_counted` below).
            //
            // For non-terminator (pure prefix) levels — e.g. `brand` in a
            // contract that has only `[brand, color]` and no standalone
            // `[brand]` index — `sub_level_is_countable_terminator` is
            // `false` and the value tree stays `NormalTree`. There's
            // nothing to count at a prefix level, and the brand-value
            // walks descend into the `color` sub-level which then carries
            // its own (potentially count-flavored) tree.
            let value_tree_type = if sub_level_is_countable_terminator {
                TreeType::CountTree
            } else {
                TreeType::NormalTree
            };

            // Wrap the property-name tree with `Element::NonCounted` iff its
            // immediate parent (the value tree at `index_path_info`) is a
            // CountTree. NonCounted-wrapping is independent of
            // `property_name_tree_type` — it only affects the *parent's*
            // count aggregation, not the wrapped element's internals.
            let wrap_property_name_tree_non_counted = parent_value_tree_is_count_tree;

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
            if wrap_property_name_tree_non_counted {
                self.batch_insert_empty_non_counted_tree_if_not_exists(
                    path_key_info.clone(),
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

                estimated_costs_only_with_layer_info.insert(
                    sub_level_index_path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        tree_type: property_name_tree_type,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            document_top_field_estimated_size as u8,
                            NoSumTrees,
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
            // Propagate the new `parent_value_tree_is_count_tree` flag
            // forward — it tracks whether the value tree we just wrote
            // (the one the sub-level will recurse INTO) is a `CountTree`.
            // That's now driven by `sub_level_is_countable_terminator`
            // (any countable tier), not just `range_countable`. Drives
            // the next level's continuation `NonCounted`-wrapping
            // decision.
            self.add_indices_for_index_level_for_contract_operations_v0(
                document_and_contract_info,
                sub_level_index_path_info,
                sub_level,
                any_fields_null,
                all_fields_null,
                sub_level_is_countable_terminator,
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
