use crate::drive::document::estimation_costs::estimated_sum_trees_for_value_tree_type::estimated_sum_trees_for_value_tree_type;
use crate::drive::document::index_level_tree_types::index_level_tree_types_with_continuation_demotion;
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
    /// v2 fixes the shared-prefix aggregate layout defect: with v1, a
    /// contract declaring an aggregating index `[a]` next to a compound
    /// index `[a, b]` registered fine but rejected every document
    /// insert, because the continuation property-name tree (`b`) could
    /// not be legally hung under `[a]`'s aggregating value trees for
    /// most flag combinations. Two changes, both consensus-affecting
    /// and therefore gated to platform v14+:
    ///
    /// 1. **Tree-type derivation** moves to the shared
    ///    [`index_level_tree_types_with_continuation_demotion`] helper,
    ///    which demotes provable count-bearing value trees
    ///    (`ProvableCountSumTree` / `ProvableCountProvableSumTree`) to
    ///    `CountSumTree` when the sub-level has continuations — grovedb
    ///    rejects count-suppressed children under provable count
    ///    parents by design, so no wrapper could ever be legal there.
    /// 2. **Continuation wrapping** goes through
    ///    [`Drive::batch_insert_empty_tree_contributing_zero_to_aggregating_parent_if_not_exists`],
    ///    which completes the parent×inner wrapper matrix that v1's
    ///    diagonal-only dispatcher rejected: non-sum continuations
    ///    under `CountSumTree` parents get `Element::NonCounted`,
    ///    non-sum continuations under sum-only parents are inserted
    ///    unwrapped (they contribute 0 to a sum naturally), and
    ///    sum-bearing continuations under count-only parents get
    ///    `Element::NonCounted` too.
    ///
    /// For every shape without a compound sibling under an aggregating
    /// terminator, both changes are bit-identical no-ops. The one
    /// intentional difference for previously-insertable shapes: a
    /// provable count-bearing value tree whose continuations were all
    /// sum-bearing could be inserted pre-v14 (grovedb's
    /// wrapper-vs-provable guard fires only when the parent merk
    /// pre-exists, which the walker's create-in-one-batch pattern never
    /// triggers); at v14+ such values get demoted `CountSumTree` value
    /// trees instead, so new writes stop depending on that unenforced
    /// guard hole. Existing provable value trees keep working — see
    /// `crate::drive::document::index_level_tree_types` for why readers
    /// are indifferent.
    ///
    /// The invariant both changes preserve: every value tree's per-axis
    /// aggregates equal exactly the contribution of its `[0]`
    /// ref-bucket, never the structural overhead of sibling compound
    /// continuations.
    ///
    /// v2 is also where the meta-schema-v3 `ranked*` grammar lands: the
    /// shared helper resolves the property-name tree through
    /// [`crate::drive::document::ranked_index_tree_type`], so a sub-level
    /// that declares a ranking axis gets the matching *indexed* tree
    /// (one ordered secondary Merk per axis) instead of the plain one.
    /// The two fixes act one level apart — ranking decides the
    /// property-name tree, the demotion decides the value trees beneath
    /// it — and a demoted `CountSumTree` value tree contributes its
    /// (count, sum) to an indexed parent exactly as the provable variant
    /// did, so ranked secondaries stay correct on shared-prefix shapes.
    /// `ranked_axes` is empty for every pre-v14 contract, making the
    /// indexed path a bit-identical no-op for them.
    ///
    /// See v1's docs for the underlying value-tree / property-name-tree
    /// design (what "countable" gates versus "range_countable", etc.);
    /// everything not listed above matches v1.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_indices_for_index_level_for_contract_operations_v2(
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
        // exact variant.
        let current_layer_tree_type = parent_value_tree_type;
        // True iff the parent value tree aggregates anything (count,
        // sum, or both) — decides whether continuation children go
        // through the zero-contribution helper or the plain one.
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
            let tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
            let property_name_tree_type = tree_types.property_name_tree_type;
            let ranked_axes = tree_types.ranked_axes.as_slice();
            let value_tree_type = tree_types.value_tree_type;

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
            if parent_value_tree_aggregates {
                // A ranked terminal level reaching this branch is
                // rejected inside the helper (it passes `ranked_axes`
                // straight through): an indexed tree can neither be
                // wrapped nor be inserted unwrapped under an
                // aggregating parent, so the shape fails closed rather
                // than silently degrading to a tree whose secondaries
                // never exist. See `INDEXED_INNER_UNWRAPPABLE` in
                // `fees::op`.
                self.batch_insert_empty_tree_contributing_zero_to_aggregating_parent_if_not_exists(
                    path_key_info.clone(),
                    parent_value_tree_type,
                    property_name_tree_type,
                    ranked_axes,
                    *storage_flags,
                    property_name_apply_type,
                    transaction,
                    previous_batch_operations,
                    batch_operations,
                    &platform_version.drive,
                )?;
            } else {
                self.batch_insert_empty_index_tree_if_not_exists(
                    path_key_info.clone(),
                    property_name_tree_type,
                    ranked_axes,
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
                        "document top field is too big for being an index",
                    )));
                }

                // The property-name layer's children are value trees of
                // type `value_tree_type` (post-demotion — matching what
                // the live path actually writes below).
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
            // Propagate the actual (post-demotion) `value_tree_type`
            // forward — the next level reads it to pick the correct
            // zero-contribution op for its own continuation children.
            self.add_indices_for_index_level_for_contract_operations_v2(
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
