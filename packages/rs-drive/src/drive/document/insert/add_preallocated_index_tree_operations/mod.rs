//! Preallocation of refersTo-determined indexOnly index trees.
//!
//! A `preallocated` index on an indexOnly document type (see
//! `dpp::data_contract::document_type::index::PREALLOCATED`) has a path that
//! is a pure function of one same-contract refersTo-referenced document:
//! every index property is either the referring property (its value is the
//! referenced document's `$id`) or a `propertyAgreement` key
//! (consensus-enforced equal to a referenced-document property at entry
//! write time). So the moment the referenced document is inserted, every
//! dynamic tree an entry referencing it will ever need — the per-value trees
//! down to the empty `0` member bucket — is fully known, and this module
//! creates them right then, charged to the referenced document's creator.
//!
//! Every tree is inserted with the same if-not-exists helpers, the same
//! tree-type derivation ([`index_level_tree_types_with_continuation_demotion`])
//! and the same estimation layers as the entry-insert walkers
//! (`add_indices_for_*_for_contract_operations` v2 and the indexOnly
//! terminal in `add_reference_for_index_level_for_contract_operations`), so
//! a preallocated tree is bit-identical to the tree the first entry's
//! create-on-insert path would have made. That fallback stays in place
//! untouched — preallocation is purely an optimization: referenced documents
//! created before a contract update introduced the flag (or whose bound
//! property values have since changed, for a mutable referenced type) simply
//! get their trees from the first entry as before.
//!
//! The counterpart lives in the delete walker: for a preallocated index,
//! removing the last member entry keeps the trees (no upward pruning), so
//! entry insertion cost stays uniform from the first entry on. See
//! `remove_reference_for_index_level_for_contract_operations`.
//!
//! Gated in place rather than by a method version: `preallocated` can only
//! be true on a PV14+ contract (the grammar rejects the keyword below
//! meta-schema v3), so this code is unreachable for every historical
//! document — the same gating the indexOnly insert branch relies on.

use crate::drive::document::estimation_costs::estimated_sum_trees_for_value_tree_type::estimated_sum_trees_for_value_tree_type;
use crate::drive::document::index_level_tree_types::{
    index_level_tree_types_with_continuation_demotion, terminal_member_tree_type,
    terminal_value_tree_type,
};
use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::document::unique_event_id;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{Index, IndexLevel, PreallocatedKeySource};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// For every preallocated index (on any indexOnly document type of the
    /// contract) whose binding targets the document type being inserted,
    /// adds the operations creating that index's dynamic trees for entries
    /// referencing the inserted document. Trees that already exist — shared
    /// prefixes with earlier referenced documents, or trees a fallback
    /// entry-insert created — are left untouched by the if-not-exists
    /// semantics.
    pub(super) fn add_preallocated_index_tree_operations_for_referring_types(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let contract = document_and_contract_info.contract;
        let target_name = document_and_contract_info.document_type.name().as_str();

        for (referring_name, referring_document_type) in contract.document_types() {
            let referring_type = referring_document_type.as_ref();
            // `preallocated` is only valid on indexOnly document types, so
            // this filter also keeps the per-insert scan trivially cheap for
            // contracts without the feature.
            if !referring_type.index_only() {
                continue;
            }
            // Flags exist to route refunds when an element is deleted, and
            // a preallocated tree has exactly one deletion path: the
            // contract's own — entry deletes retain it by design (the
            // delete walker's no-prune rule), and entry-level deletability
            // (`documents_can_be_deleted`) never reaches it. So unlike the
            // entry walkers' rule, flags ride only when the contract is
            // deletable; on a permanent contract they would be dead bytes
            // charged to the referenced document's creator on every tree.
            // (Fallback-created trees may carry entry-rule flags from
            // before the no-prune retention — harmless: unrefundable flags
            // are inert, and if-not-exists inserts never rewrite them.)
            let storage_flags = if contract.config().can_be_deleted() {
                document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_storage_flags_ref()
            } else {
                None
            };
            for index in referring_type.indexes().values() {
                if !index.preallocated {
                    continue;
                }
                for binding in index
                    .preallocation_bindings(referring_type.flattened_properties(), contract.id())
                {
                    if binding.target_document_type_name != target_name {
                        continue;
                    }
                    self.add_preallocated_index_tree_operations_for_binding(
                        document_and_contract_info,
                        referring_name,
                        referring_type.index_structure(),
                        index,
                        &binding.key_sources,
                        storage_flags,
                        previous_batch_operations,
                        estimated_costs_only_with_layer_info,
                        transaction,
                        batch_operations,
                        platform_version,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Adds the operations preallocating ONE index's trees for entries
    /// referencing the inserted document: walks the referring type's
    /// [`IndexLevel`] along the index's properties, resolving each path key
    /// from the inserted document per the binding's key sources, and creates
    /// each property-name tree, value tree and the terminal `0` member
    /// bucket with exactly the tree types and estimation layers the
    /// entry-insert walkers use.
    #[allow(clippy::too_many_arguments)]
    fn add_preallocated_index_tree_operations_for_binding(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        referring_type_name: &str,
        referring_index_structure: &IndexLevel,
        index: &Index,
        key_sources: &[PreallocatedKeySource],
        storage_flags: Option<&StorageFlags>,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        let contract = document_and_contract_info.contract;
        let target_document_type = document_and_contract_info.document_type;
        let document_info = &document_and_contract_info.owned_document_info.document_info;
        let event_id = unique_event_id();

        let contract_document_type_path =
            contract_document_type_path_vec(contract.id_ref().as_bytes(), referring_type_name);

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // The referring doctype tree layer — mirror of the entry-insert
            // walkers' top-level entry, but for the referring type.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(contract_document_type_path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(
                        referring_index_structure.sub_levels().len() as u32 + 1,
                    ),
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        let mut current_level = referring_index_structure;
        let mut index_path_info: Option<PathInfo<0>> = None;
        // The value tree type of the level above, deciding whether the next
        // property-name tree needs the zero-contribution wrapper — exactly
        // the `parent_value_tree_type` the recursive walker threads through.
        let mut parent_value_tree_type = TreeType::NormalTree;

        for (index_property, key_source) in index.properties.iter().zip(key_sources.iter()) {
            let property_name = index_property.name.as_str();
            let sub_level = current_level
                .sub_levels()
                .get(property_name)
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a preallocated index's property must exist in the index structure",
                )))?;
            let tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
            let property_name_tree_type = tree_types.property_name_tree_type;
            let ranked_axes = tree_types.ranked_axes.as_slice();
            let value_tree_type = tree_types.value_tree_type;

            // Resolve this level's value key from the inserted (referenced)
            // document. `$id` for the referring property; the agreed
            // referenced property otherwise — validated at contract
            // registration to share one value kind with the referring side,
            // so the encoded bytes match what an entry insert would write.
            let source_property = match *key_source {
                PreallocatedKeySource::ReferencedDocumentId => "$id",
                PreallocatedKeySource::ReferencedDocumentProperty(referenced) => referenced,
            };
            let Some(value_key) = document_info.get_raw_for_document_type(
                source_property,
                target_document_type,
                document_and_contract_info.owned_document_info.owner_id,
                Some((sub_level, event_id)),
                platform_version,
            )?
            else {
                // The referenced document does not carry the bound property
                // (it is optional there): no entry can ever agree with it,
                // and if that changes on an update the entry-insert fallback
                // creates the trees. Nothing to preallocate.
                return Ok(());
            };
            if value_key.is_empty() {
                // A null value takes the null index layout on the entry
                // side; leave that (never-preallocatable) shape entirely to
                // the fallback.
                return Ok(());
            }

            let mut path_info = match index_path_info.take() {
                None => {
                    // First property: its property-name tree is static —
                    // created at contract registration — so only enter it.
                    let mut index_path = contract_document_type_path.clone();
                    index_path.push(Vec::from(property_name.as_bytes()));
                    if document_info.is_document_size() {
                        PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(index_path))
                    } else {
                        PathInfo::PathAsVec::<0>(index_path)
                    }
                }
                Some(mut path_info) => {
                    // Deeper property-name trees are dynamic: create this
                    // one inside the parent value tree, wrapped to
                    // contribute zero when that parent aggregates — the
                    // same dispatch the recursive walker uses.
                    let property_name_apply_type = if estimated_costs_only_with_layer_info.is_none()
                    {
                        BatchInsertTreeApplyType::StatefulBatchInsertTree
                    } else {
                        BatchInsertTreeApplyType::StatelessBatchInsertTree {
                            in_tree_type: parent_value_tree_type,
                            tree_type: property_name_tree_type,
                            flags_len: storage_flags
                                .map(|s| s.serialized_size())
                                .unwrap_or_default(),
                        }
                    };
                    let path_key_info =
                        KeyRef(property_name.as_bytes()).add_path_info(path_info.clone());
                    if !matches!(parent_value_tree_type, TreeType::NormalTree) {
                        self.batch_insert_empty_tree_contributing_zero_to_aggregating_parent_if_not_exists(
                            path_key_info,
                            parent_value_tree_type,
                            property_name_tree_type,
                            ranked_axes,
                            storage_flags,
                            property_name_apply_type,
                            transaction,
                            previous_batch_operations,
                            batch_operations,
                            drive_version,
                        )?;
                    } else {
                        self.batch_insert_empty_index_tree_if_not_exists(
                            path_key_info,
                            property_name_tree_type,
                            ranked_axes,
                            storage_flags,
                            property_name_apply_type,
                            transaction,
                            previous_batch_operations,
                            batch_operations,
                            drive_version,
                        )?;
                    }
                    path_info.push(KeyRef(property_name.as_bytes()))?;
                    path_info
                }
            };

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                // The property-name layer: children are value trees keyed by
                // the source property's values (32 bytes for `$id`).
                let value_key_estimated_size = match key_source {
                    PreallocatedKeySource::ReferencedDocumentId => DEFAULT_HASH_SIZE_U8 as u16,
                    PreallocatedKeySource::ReferencedDocumentProperty(_) => document_info
                        .get_estimated_size_for_document_type(
                            source_property,
                            target_document_type,
                            platform_version,
                        )?,
                };
                if value_key_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "referenced document field is too big for being an index",
                    )));
                }
                estimated_costs_only_with_layer_info.insert(
                    path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        tree_type: property_name_tree_type,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            value_key_estimated_size as u8,
                            estimated_sum_trees_for_value_tree_type(value_tree_type),
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            // The value tree for the resolved key.
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
            let path_key_info = value_key.clone().add_path_info(path_info.clone());
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info,
                value_tree_type,
                storage_flags,
                value_apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                drive_version,
            )?;
            path_info.push(value_key)?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                // The value tree's own layer: its children are the `0`
                // member bucket plus any continuation property-name trees.
                estimated_costs_only_with_layer_info.insert(
                    path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        tree_type: value_tree_type,
                        estimated_layer_count: ApproximateElements(
                            sub_level.sub_levels().len() as u32 + 1,
                        ),
                        estimated_layer_sizes: AllSubtrees(
                            DEFAULT_HASH_SIZE_U8,
                            NoSumTrees,
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            index_path_info = Some(path_info);
            parent_value_tree_type = value_tree_type;
            current_level = sub_level;
        }

        let mut path_info = index_path_info.ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution("a preallocated index must have properties"),
        ))?;

        // The terminal `0` member bucket — mirror of the tree-creation half
        // of `add_index_only_terminal_item_operations` (the member entry
        // itself is each entry insert's own write).
        let level_info = current_level.has_index_with_type().ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "a preallocated index must terminate at its last property",
            ),
        ))?;
        let member_tree_type = terminal_member_tree_type(level_info);
        let member_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                // The `0` tree's parent is the index's value tree — same
                // aggregate-aware claim the entry-insert terminal makes.
                in_tree_type: terminal_value_tree_type(level_info),
                tree_type: member_tree_type,
                flags_len: storage_flags
                    .map(|s| s.serialized_size())
                    .unwrap_or_default(),
            }
        };
        let path_key_info = KeyRef(&[0]).add_path_info(path_info.clone());
        self.batch_insert_empty_tree_if_not_exists(
            path_key_info,
            member_tree_type,
            storage_flags,
            member_apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            path_info.push(Key(vec![0]))?;
            // Same per-entry padding (and sum-item worst case) the
            // entry-insert terminal claims for this layer — see
            // `add_index_only_terminal_item_operations`.
            const INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE: u32 =
                crate::drive::document::INDEX_ONLY_ROW_COMMITMENT_SIZE + 32;
            let estimated_value_size = if level_info.summable.is_some() {
                INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE + 10
            } else {
                INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE
            };
            estimated_costs_only_with_layer_info.insert(
                path_info.convert_to_key_info_path(),
                EstimatedLayerInformation {
                    tree_type: member_tree_type,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: AllItems(
                        DEFAULT_HASH_SIZE_U8,
                        estimated_value_size,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        Ok(())
    }
}
