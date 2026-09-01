use crate::drive::document::unique_event_id;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;

use crate::util::grove_operations::BatchInsertTreeApplyType;

use crate::drive::Drive;
use crate::util::object_size_info::{
    DocumentAndContractInfo, DocumentInfoV0Methods, DriveKeyInfo, PathInfo,
};

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};

use dpp::version::PlatformVersion;

use crate::drive::document::estimation_costs::estimated_sum_trees_for_value_tree_type::estimated_sum_trees_for_value_tree_type;
use crate::drive::document::index_level_tree_types::{
    index_level_tree_types_with_continuation_demotion, time_range_index_keys,
};
use crate::drive::document::paths::contract_document_type_path_vec;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds indices for the top index level and calls for lower levels.
    ///
    /// v2 derives the per-sub-level tree types through the shared
    /// [`index_level_tree_types_with_continuation_demotion`] helper
    /// instead of v1's inline tables. The only behavioral difference is
    /// the continuation demotion: a top-level property that both
    /// terminates a countable+summable index with a range flag AND
    /// prefixes a compound index gets `CountSumTree` value trees
    /// instead of the provable variants, because grovedb rejects
    /// count-suppressed continuation children under provable count
    /// parents. Shapes without continuations (and all shapes insertable
    /// under v1) produce bit-identical operations. Part of the platform
    /// v14 shared-prefix aggregate fix — see
    /// [`Drive::add_indices_for_index_level_for_contract_operations_v2`]
    /// for the full story.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_indices_for_top_index_level_for_contract_operations_v2(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        let index_level = &document_and_contract_info.document_type.index_structure();
        let contract = document_and_contract_info.contract;
        let event_id = unique_event_id();
        let document_type = document_and_contract_info.document_type;
        let storage_flags = if document_type.documents_mutable()
                || contract.config().can_be_deleted()
                // indexOnly entries ARE the rows: they are deleted (and
                // refunded) whenever the doctype allows deletion, so their
                // flags must ride even though the type is immutable and the
                // contract may not be deletable. PV14-only (`index_only()`
                // cannot be true below meta-schema v3), so historical
                // replay is untouched.
                || (document_type.index_only() && document_type.documents_can_be_deleted())
        {
            document_and_contract_info
                .owned_document_info
                .document_info
                .get_storage_flags_ref()
        } else {
            None //there are no need for storage flags if documents are not mutable and contract can not be deleted
        };

        // we need to construct the path for documents on the contract
        // the path is
        //  * Document and DataContract root tree
        //  * DataContract ID recovered from document
        //  * 0 to signify Documents and notDataContract
        let contract_document_type_path = contract_document_type_path_vec(
            document_and_contract_info.contract.id_ref().as_bytes(),
            document_and_contract_info.document_type.name(),
        );

        let sub_level_index_count = index_level.sub_levels().len() as u32;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(contract_document_type_path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(sub_level_index_count + 1),
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        // next we need to store a reference to the document for each index
        for (name, sub_level) in index_level.sub_levels() {
            // The top-level property-name tree is created once, at
            // contract registration — this walker never writes it, so it
            // has no use for `tree_types.ranked_axes`. The resolved type
            // still has to include the meta-schema-v3 ranked upgrade: it
            // describes the tree we insert INTO (the stateless apply
            // type's `in_tree_type`) and stamps the estimation layer, and
            // both must agree with what `insert_contract_operations_v0`
            // laid down.
            let tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
            let property_name_tree_type = tree_types.property_name_tree_type;
            let value_tree_type = tree_types.value_tree_type;

            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path: Vec<Vec<u8>> = contract_document_type_path.clone();
            index_path.push(Vec::from(name.as_bytes()));

            // The level key is the path segment; the document value is read
            // from the *source property*. They coincide except on a
            // time-range level, whose key is the property name qualified
            // with the grid (`TimeRangeTransform::storage_key`) while the
            // timestamp still lives under the bare property name.
            let property_name = sub_level
                .time_range()
                .map(|transform| transform.source.as_str())
                .unwrap_or(name.as_str());

            // with the example of the dashpay contract's first index
            // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId
            let document_top_field = match document_and_contract_info
                .owned_document_info
                .document_info
                .get_raw_for_document_type(
                    property_name,
                    document_type,
                    document_and_contract_info.owned_document_info.owner_id,
                    Some((sub_level, event_id)),
                    platform_version,
                )? {
                Some(document_top_field) => document_top_field,
                // An unrequired top-level property on an indexOnly type is a
                // skipIfAbsent index's trigger (the parser admits no other
                // optional property), and every index through this branch is
                // such an index — an absent trigger writes NOTHING: no
                // property-name tree, no descent, no entries. Skipping
                // before any operation is emitted is what keeps the branch
                // free of stranded prefix trees; the probes mirror this
                // exact condition in `index_only_entry_paths_and_key`. A
                // create's estimation dry-run reads the real document (so it
                // skips exactly when apply skips), and the timestamp of a
                // bucketed level can never land here (its source is
                // `$createdAt`, required whenever indexed).
                None if document_type.index_only()
                    && !document_type.required_fields().contains(property_name) =>
                {
                    continue;
                }
                // A stored type's absent value keeps its null-layout empty
                // key.
                None => DriveKeyInfo::default(),
            };

            // here we are inserting the value tree (per distinct property value)
            // under the top-level property-name tree. The top-level property-name
            // tree itself is created at contract setup, so the apply_type's
            // `in_tree_type` reflects whichever variant the contract setup used.
            // Same for every bucket key when this is a time-range node.
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

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_estimated_size_for_document_type(
                        property_name,
                        document_type,
                        platform_version,
                    )?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document field is too big for being an index",
                    )));
                }

                // On this level we will have all the user defined values
                // for the paths. Children at this property-name layer
                // are value trees of type `value_tree_type` (post-
                // demotion — matching what the live path actually
                // writes above).
                estimated_costs_only_with_layer_info.insert(
                    KeyInfoPath::from_known_owned_path(index_path.clone()),
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

            let any_fields_null = document_top_field.is_empty();
            let all_fields_null = document_top_field.is_empty();

            // A time-range first-property node expands the document's single
            // timestamp into one index entry per overlapping range bucket (the
            // bucket *start*, encoded exactly like the timestamp). A normal
            // property keeps its single key. The entry-key rule (null keeps
            // its single null entry, pre-origin timestamps produce no entries,
            // undecodable values keep their raw key) lives in ONE place —
            // [`TimeRangeTransform::entry_keys_for_raw`] — shared with the
            // delete and update walkers so the three can never disagree.
            let index_keys: Vec<DriveKeyInfo> = time_range_index_keys(
                sub_level.time_range(),
                document_top_field,
                // A validated contract cannot exceed this; the clamp only
                // bounds estimation work for unvalidated transforms. The
                // `unwrap_or(1)` arm is a protocol version without
                // time-range indexes, where no transform can exist.
                platform_version
                    .system_limits
                    .max_time_range_overlap_factor
                    .unwrap_or(1),
            );

            let bucket_count = index_keys.len();
            for (bucket, index_key) in index_keys.into_iter().enumerate() {
                // The zero will not matter here, because the PathKeyInfo is variable
                let path_key_info = index_key.clone().add_path::<0>(index_path.clone());
                let newly_created_bucket = self.batch_insert_empty_tree_if_not_exists(
                    path_key_info,
                    value_tree_type,
                    storage_flags,
                    value_apply_type,
                    transaction,
                    previous_batch_operations,
                    batch_operations,
                    drive_version,
                )?;

                // TTL cleanup rides the bucket-creating write: a new bucket
                // means time rolled forward, so buckets behind the horizon
                // are dropped — capped per write, oldest first. Steady state
                // is one-for-one (one new bucket per step, one expiring);
                // the cap amortizes catch-up after a quiet spell. Stateful
                // only: the estimation dry run neither reads state nor
                // prices drops (their cost class is the triggering write's
                // processing, bounded by the cap — and O(1) per drop once
                // grovedb#848 replaces the placeholder).
                if newly_created_bucket && estimated_costs_only_with_layer_info.is_none() {
                    if let Some(transform) = sub_level.time_range() {
                        if transform.ttl_seconds.is_some() {
                            if let Some(max_drops) = platform_version
                                .system_limits
                                .max_time_range_expired_bucket_drops_per_write
                            {
                                self.drop_expired_time_range_buckets(
                                    transform,
                                    &index_path,
                                    block_time_ms,
                                    max_drops,
                                    transaction,
                                    batch_operations,
                                    platform_version,
                                )?;
                            }
                        }
                    }
                }

                // The final bucket takes ownership of `index_path`; earlier
                // buckets (only a time-range fan-out has more than one)
                // clone it.
                let own_index_path = if bucket + 1 == bucket_count {
                    std::mem::take(&mut index_path)
                } else {
                    index_path.clone()
                };
                let mut index_path_info = if document_and_contract_info
                    .owned_document_info
                    .document_info
                    .is_document_size()
                {
                    // This is a stateless operation
                    PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(own_index_path))
                } else {
                    PathInfo::PathAsVec::<0>(own_index_path)
                };

                // we push the actual value of the index path
                index_path_info.push(index_key)?;
                // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>

                // Propagate the exact (post-demotion) `value_tree_type` we
                // just inserted forward as the recursive level's
                // `parent_value_tree_type` so its continuation children pick
                // the right zero-contribution op.
                self.add_indices_for_index_level_for_contract_operations(
                    document_and_contract_info,
                    index_path_info,
                    sub_level,
                    any_fields_null,
                    all_fields_null,
                    value_tree_type,
                    previous_batch_operations,
                    &storage_flags,
                    estimated_costs_only_with_layer_info,
                    event_id,
                    transaction,
                    batch_operations,
                    platform_version,
                )?;
            }
        }
        Ok(())
    }
}
