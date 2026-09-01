use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use crate::drive::document::estimation_costs::estimated_sum_trees_for_value_tree_type::estimated_sum_trees_for_value_tree_type;
use crate::drive::document::index_level_tree_types::{
    index_level_tree_types_with_continuation_demotion, time_range_index_keys,
};
use crate::drive::document::time_range_ttl::entry_key_bucket_start;
use crate::drive::document::unique_event_id;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;

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

use crate::drive::document::paths::contract_document_type_path_vec;
use dpp::version::PlatformVersion;

impl Drive {
    /// Removes indices for the top index level and calls for lower levels.
    ///
    /// v2 derives tree types through the shared
    /// [`index_level_tree_types_with_continuation_demotion`] helper so
    /// the estimation layer info describes the exact on-disk shape the
    /// v2 insert walker writes — including the continuation demotion of
    /// provable count-bearing value trees to `CountSumTree`. Must stay
    /// in lockstep with
    /// [`Drive::add_indices_for_top_index_level_for_contract_operations_v2`];
    /// part of the platform v14 shared-prefix aggregate fix.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_indices_for_top_index_level_for_contract_operations_v2(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        previous_batch_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let document_type = document_and_contract_info.document_type;
        let index_level = document_type.index_structure();
        let contract = document_and_contract_info.contract;
        let event_id = unique_event_id();
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
        //  * Document andDataContract root tree
        //  *DataContract ID recovered from document
        //  * 0 to signify Documents and notDataContract
        let contract_document_type_path = contract_document_type_path_vec(
            document_and_contract_info.contract.id_ref().as_bytes(),
            document_and_contract_info.document_type.name().as_str(),
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
            // The delete walker writes nothing itself, but its
            // estimation layers must describe the tree the insert path
            // actually laid down — including the meta-schema-v3 ranked
            // upgrade of the property-name tree — or dry-run delete fees
            // drift from applied ones on ranked indexes.
            let tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
            let property_name_tree_type = tree_types.property_name_tree_type;
            let value_tree_type = tree_types.value_tree_type;

            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path: Vec<Vec<u8>> = contract_document_type_path.clone();
            index_path.push(Vec::from(name.as_bytes()));

            // The level key is the path segment; the document value is read
            // from the *source property* — they differ on a time-range
            // level, whose key is grid-qualified
            // (`TimeRangeTransform::storage_key`) while the timestamp lives
            // under the bare property name. Mirrors the insert walker.
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
                // skipIfAbsent index's trigger: the insert walker wrote no
                // entries through this branch for a trigger-absent document,
                // so its delete removes none — mirroring the insert skip is
                // what keeps delete-by-values exact. The delete's estimation
                // dry-run runs on a worst-case document info that always
                // resolves a value, so estimation sweeps this branch as
                // written — a deliberate over-estimate that keeps the dry
                // run an upper bound.
                None if document_type.index_only()
                    && !document_type.required_fields().contains(property_name) =>
                {
                    continue;
                }
                // A stored type's absent value keeps its null-layout empty
                // key.
                None => DriveKeyInfo::default(),
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
                        "document top field is too big for being an index",
                    )));
                }

                // The property-name layer's children are value trees of
                // type `value_tree_type` (post-demotion — matching what
                // the v2 insert walker actually writes).
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

            // Mirror the insert side's time-range fan-out: a time-range
            // first-property node removes one index entry per overlapping
            // range bucket the document's timestamp fell into. The keys are
            // recomputed deterministically through the same shared helper the
            // insert walker uses, so they match exactly what insert wrote —
            // including the null case (single null entry) and the pre-origin
            // case (no entries on either side).
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
                // TTL: an expired bucket may already have been dropped
                // entirely (this document's entries went with it — skip),
                // or stand PARTIALLY drained (drainage removes whole `[0]`
                // and group value trees before the bucket): removal then
                // proceeds, but at full-path granularity — the deeper
                // walkers skip any entry whose path the drain already
                // took. Live buckets behave exactly as before. Stateful
                // reads have no place in the estimation dry run, which
                // processes every bucket — the upper bound.
                let mut skip_missing_expired_entry = false;
                if estimated_costs_only_with_layer_info.is_none() {
                    if let Some(transform) = sub_level.time_range() {
                        let entry_key_bytes = match &index_key {
                            DriveKeyInfo::Key(key) => Some(key.as_slice()),
                            DriveKeyInfo::KeyRef(key) => Some(*key),
                            DriveKeyInfo::KeySize(_) => None,
                        };
                        if let Some(entry_key_bytes) = entry_key_bytes {
                            let expired = entry_key_bucket_start(entry_key_bytes)
                                .zip(transform.expiry_horizon_ms(block_time_ms))
                                .is_some_and(|(start, horizon)| start < horizon);
                            if expired {
                                if !self.time_range_entry_is_removable(
                                    transform,
                                    entry_key_bytes,
                                    block_time_ms,
                                    &index_path,
                                    transaction,
                                    platform_version,
                                )? {
                                    continue;
                                }
                                skip_missing_expired_entry = true;
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

                self.remove_indices_for_index_level_for_contract_operations(
                    document_and_contract_info,
                    index_path_info,
                    sub_level,
                    any_fields_null,
                    all_fields_null,
                    value_tree_type,
                    &storage_flags,
                    previous_batch_operations,
                    estimated_costs_only_with_layer_info,
                    skip_missing_expired_entry,
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
