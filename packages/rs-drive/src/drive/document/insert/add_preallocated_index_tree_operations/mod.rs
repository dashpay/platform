//! Preallocation of refersTo-determined indexOnly index trees.
//!
//! A `preallocated` index on an indexOnly document type (see
//! `dpp::data_contract::document_type::index::PREALLOCATED`) has a path that
//! is a pure function of one same-contract refersTo-referenced document (a
//! `permanentDocument` one, or a `moderatedDocument` one whose removal record
//! keeps every key of the path):
//! every index property is either the referring property (its value is the
//! referenced document's `$id`) or a `where` referring value
//! (consensus-enforced equal to a referenced-document property at entry
//! write time). So the moment the referenced document is inserted, every
//! dynamic tree an entry referencing it will ever need — the per-value trees
//! down to the empty `0` member bucket — is fully known, and this module
//! creates them right then, charged to the referenced document's creator.
//! A typed array a `where` makes equal to a referenced typed array (protocol
//! version 14) holds the referenced document's list in every entry, which
//! enters it under each element: its level takes one value tree per distinct
//! element of that list, each with every tree below it, and an empty or
//! absent list preallocates nothing (every index naming the array skips it).
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
//! Doubly gated: the caller is `add_document_for_contract_operations_v1`,
//! selected only by protocol v14's method table
//! (`DRIVE_DOCUMENT_METHOD_VERSIONS_V4`), and `preallocated` itself cannot
//! be true below meta-schema v3 — so the platform-version snapshot names
//! the active insert semantics AND historical documents can never reach
//! this code. The delete-side counterpart is versioned the same way
//! (`remove_reference_for_index_level_for_contract_operations_v1`).

use crate::drive::document::bound_value_fits_referring_property;
use crate::drive::document::estimation_costs::estimated_sum_trees_for_value_tree_type::estimated_sum_trees_for_value_tree_type;
use crate::drive::document::index_level_tree_types::{
    continuation_contributes_zero, index_level_tree_types_with_continuation_demotion,
    terminal_member_tree_type, terminal_value_tree_type,
};
use crate::drive::document::index_only::index_only_terminal_max_key_size;
use crate::drive::document::index_only_item_estimated_value_size;
use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::document::preallocation_bindings_targeting;
use crate::drive::document::unique_event_id;
use crate::drive::document::{
    referenced_list_tree_keys_for_referring_property, referenced_tree_key_for_referring_property,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::{LowLevelDriveOperation, LowLevelDriveOperationTreeTypeConverter};
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::util::object_size_info::{
    DocumentAndContractInfo, DocumentInfoV0Methods, DriveKeyInfo, PathInfo, PathKeyInfo,
};
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{
    DocumentReferenceKind, DocumentTypeRef, Index, IndexLevel, PreallocatedKeySource,
    PreallocationBinding,
};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

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
        // Flags exist to route refunds when an element is deleted, and a
        // preallocated tree has exactly one deletion path: the contract's own
        // — entry deletes retain it by design (the delete walker's no-prune
        // rule), and entry-level deletability (`documents_can_be_deleted`)
        // never reaches it. So unlike the entry walkers' rule, flags ride only
        // when the contract is deletable; on a permanent contract they would
        // be dead bytes charged to the referenced document's creator on every
        // tree. (Fallback-created trees may carry entry-rule flags from before
        // the no-prune retention — harmless: unrefundable flags are inert, and
        // if-not-exists inserts never rewrite them.)
        let storage_flags = if contract.config().can_be_deleted() {
            document_and_contract_info
                .owned_document_info
                .document_info
                .get_storage_flags_ref()
        } else {
            None
        };
        // Each binding walks its own index, so two preallocated indexes sharing
        // leading properties reach the same trees: each binding's inserts are
        // checked against the operations the earlier ones queued (after the
        // earlier documents' of the batch), so a tree is queued once, as the
        // entry walkers, walking the shared index levels once, do. A tree
        // already in state is not queued, so each binding reaching it reads its
        // existence again (a billed read per binding, where the entry walkers
        // read it once); under a level keyed by the elements of a list that is
        // a read per binding and element, when two preallocated indexes start
        // with the same list. Over-priced, never under-priced: each read is a
        // billed operation the estimate also prices.
        let mut queued = Vec::new();
        for (referring_type, index, binding) in
            preallocation_bindings_targeting(contract, document_and_contract_info.document_type)
        {
            let mut binding_operations = Vec::new();
            let result = match previous_batch_operations {
                Some(previous) => {
                    // The earlier documents' operations and this document's,
                    // checked as one queue; inserts never leave it, so ours
                    // are its tail again afterwards.
                    let ours = queued.len();
                    previous.append(&mut queued);
                    let result = self.add_preallocated_index_tree_operations_for_binding(
                        document_and_contract_info,
                        referring_type,
                        index,
                        &binding,
                        storage_flags,
                        &mut Some(&mut **previous),
                        estimated_costs_only_with_layer_info,
                        transaction,
                        &mut binding_operations,
                        platform_version,
                    );
                    queued = previous.split_off(previous.len() - ours);
                    result
                }
                None => self.add_preallocated_index_tree_operations_for_binding(
                    document_and_contract_info,
                    referring_type,
                    index,
                    &binding,
                    storage_flags,
                    &mut Some(&mut queued),
                    estimated_costs_only_with_layer_info,
                    transaction,
                    &mut binding_operations,
                    platform_version,
                ),
            };
            result?;
            queued.append(&mut binding_operations);
        }
        batch_operations.append(&mut queued);
        Ok(())
    }

    /// Adds the operations preallocating ONE index's trees for entries
    /// referencing the inserted document: walks the referring type's
    /// [`IndexLevel`] along
    /// the index's properties, resolving each path key from the inserted
    /// document per the binding's key sources, and creates each
    /// property-name tree, value tree and the terminal `0` member bucket
    /// with exactly the tree types and estimation layers the entry-insert
    /// walkers use. A level keyed by the elements of a list continues the
    /// walk under each element's value tree.
    #[allow(clippy::too_many_arguments)]
    fn add_preallocated_index_tree_operations_for_binding(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        referring_type: DocumentTypeRef,
        index: &Index,
        binding: &PreallocationBinding,
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
        let document_info = &document_and_contract_info.owned_document_info.document_info;
        let event_id = unique_event_id();

        let referring_index_structure = referring_type.index_structure();
        let contract_document_type_path =
            contract_document_type_path_vec(contract.id_ref().as_bytes(), referring_type.name());

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

        // Resolve EVERY path key before emitting a single operation: a
        // missing or null bound value at ANY level means no entry can ever
        // agree with this document, so nothing must be preallocated — and
        // bailing mid-walk would leave the earlier levels' tree inserts
        // already sitting in the batch. Only the stateful path can skip
        // this way; in estimation mode every key resolves to a worst-case
        // size, so the dry-run always sweeps the full path. A level keyed by
        // the elements of a list takes one key per element, and an empty list
        // (which every agreeing entry holds, and every index naming the array
        // skips) preallocates nothing either.
        let mut resolved_levels = Vec::with_capacity(index.properties.len());
        let mut current_level = referring_index_structure;
        for (index_property, key_source) in index.properties.iter().zip(binding.key_sources.iter())
        {
            let property_name = index_property.name.as_str();
            let sub_level = current_level
                .sub_levels()
                .get(property_name)
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a preallocated index's property must exist in the index structure",
                )))?;
            let value_keys = match *key_source {
                PreallocatedKeySource::ReferencedDocumentListElements(referenced) => {
                    match document_info.get_borrowed_document() {
                        // Without a document the worst case is `maxItems`
                        // distinct elements, each sized as an element of the
                        // referring array, as an entry insert sizes them
                        None => document_info.get_raw_array_elements_for_document_type(
                            property_name,
                            referring_type,
                            Some((sub_level, event_id)),
                            platform_version,
                        )?,
                        Some(document) => {
                            let Some(keys) = referenced_list_tree_keys_for_referring_property(
                                document,
                                referenced,
                                referring_type,
                                property_name,
                                platform_version,
                            )?
                            else {
                                // No list a referring document can hold:
                                // nothing to preallocate
                                return Ok(());
                            };
                            keys.into_iter().map(Key).collect()
                        }
                    }
                }
                PreallocatedKeySource::ReferencedDocumentId
                | PreallocatedKeySource::ReferencedDocumentProperty(_) => {
                    let Some(value_key) = preallocated_single_key(
                        document_and_contract_info,
                        referring_type,
                        property_name,
                        sub_level,
                        *key_source,
                        event_id,
                        platform_version,
                    )?
                    else {
                        return Ok(());
                    };
                    vec![value_key]
                }
            };
            if value_keys.is_empty() {
                return Ok(());
            }
            resolved_levels.push((property_name, sub_level, value_keys));
            current_level = sub_level;
        }
        let terminal_level = current_level;

        let stateful = estimated_costs_only_with_layer_info.is_none();
        let flags_len = storage_flags
            .map(|s| s.serialized_size())
            .unwrap_or_default();

        // The trees reached so far, each the path the walk continues under,
        // and whether this walk created the tree at its end: nothing can sit
        // under a tree it created, so a stateful insert of a plain tree there
        // skips the existence read (an estimate still prices the read, so it
        // bounds every stateful write). It starts at the document type's tree,
        // which holds the static first property-name tree. One path, except
        // below a level keyed by the elements of a list, which continues once
        // under each element's value tree: each tree is then queued once,
        // under the one path it belongs to.
        let document_type_path_info = if document_info.is_document_size() {
            PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(
                contract_document_type_path.clone(),
            ))
        } else {
            PathInfo::PathAsVec::<0>(contract_document_type_path.clone())
        };
        let mut frontier = vec![(document_type_path_info, false)];
        // The value tree type of the level above, deciding whether the next
        // property-name tree needs the zero-contribution wrapper — exactly
        // the `parent_value_tree_type` the recursive walker threads through.
        let mut parent_value_tree_type = TreeType::NormalTree;
        // Whether the level above is a prefix-ranking chain level (a
        // grouping or propagating level of a count, sum or average chain):
        // its value trees aggregate exactly their single continuation, so
        // the continuation is inserted unwrapped and contributes — the same
        // inversion the entry-insert walkers apply.
        let mut parent_counts_continuations = false;

        for (position, (property_name, sub_level, value_keys)) in
            resolved_levels.into_iter().enumerate()
        {
            let tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
            let property_name_tree_type = tree_types.property_name_tree_type;
            let ranked_axes = tree_types.ranked_axes.as_slice();
            let value_tree_type = tree_types.value_tree_type;
            let property_name_apply_type = if stateful {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: parent_value_tree_type,
                    tree_type: property_name_tree_type,
                    flags_len,
                }
            };
            let value_apply_type = if stateful {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: property_name_tree_type,
                    tree_type: value_tree_type,
                    flags_len,
                }
            };
            // A count-exempt sibling branch re-inverts the chain-level choice
            // per child (a preallocated index may itself be the plain
            // sibling): its branch tree is zero-wrapped even under a chain
            // level that counts its own continuation — matching the
            // entry-insert walkers.
            let contributes_zero = continuation_contributes_zero(
                parent_value_tree_type,
                parent_counts_continuations,
                sub_level,
            );

            // This level's property-name tree under each path so far, and
            // whether this walk created it: a tree or counter under it cannot
            // exist yet.
            let mut property_name_paths = Vec::with_capacity(frontier.len());
            for (mut path_info, _) in std::mem::take(&mut frontier) {
                // The first property's property-name tree is static —
                // created at contract registration — so only enter it.
                // Deeper ones are dynamic: create this one inside the parent
                // value tree, wrapped to contribute zero when that parent
                // aggregates — the same dispatch the recursive walker uses.
                let created = if position == 0 {
                    false
                } else {
                    let path_key_info =
                        KeyRef(property_name.as_bytes()).add_path_info(path_info.clone());
                    if contributes_zero {
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
                        )?
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
                        )?
                    }
                };
                path_info.push(KeyRef(property_name.as_bytes()))?;
                property_name_paths.push((path_info, created));
            }

            // A summableOffCountIndex index keeps its group's counter at the value
            // position of its last property: preallocating it is creating the
            // counter at zero, in place of the value tree and its `0` bucket,
            // one per element when the property is a list.
            // Nothing continues below it, so this is the last level.
            if sub_level.summable_off_count_index_info().is_some() {
                for (path_info, property_name_tree_created) in property_name_paths {
                    self.add_summable_off_count_zero_counter_operations(
                        path_info,
                        value_keys.clone(),
                        property_name_tree_type,
                        property_name_tree_created
                            || binding.kind == DocumentReferenceKind::Permanent,
                        storage_flags,
                        previous_batch_operations,
                        || {
                            document_info.get_estimated_size_for_document_type(
                                property_name,
                                referring_type,
                                platform_version,
                            )
                        },
                        estimated_costs_only_with_layer_info,
                        transaction,
                        batch_operations,
                        platform_version,
                    )?;
                }
                return Ok(());
            }

            // The property-name layer: children are value trees keyed by the
            // referring property's values, sized exactly as an entry insert
            // sizes this layer (32 bytes for the reference property). A
            // referenced document only keys it with a value that fits that
            // property, so a wider referenced property does not widen the
            // estimate.
            let value_key_estimated_size = if stateful {
                0
            } else {
                let size = document_info.get_estimated_size_for_document_type(
                    property_name,
                    referring_type,
                    platform_version,
                )?;
                u8::try_from(size).map_err(|_| {
                    Error::Fee(FeeError::Overflow(
                        "document field is too big for being an index",
                    ))
                })?
            };
            for (path_info, property_name_tree_created) in property_name_paths {
                if let Some(estimated_costs_only_with_layer_info) =
                    estimated_costs_only_with_layer_info
                {
                    estimated_costs_only_with_layer_info.insert(
                        path_info.clone().convert_to_key_info_path(),
                        EstimatedLayerInformation {
                            tree_type: property_name_tree_type,
                            estimated_layer_count: PotentiallyAtMaxElements,
                            estimated_layer_sizes: AllSubtrees(
                                value_key_estimated_size,
                                estimated_sum_trees_for_value_tree_type(value_tree_type),
                                storage_flags.map(|s| s.serialized_size()),
                            ),
                        },
                    );
                }

                for value_key in &value_keys {
                    // The value tree for the resolved key.
                    let path_key_info = value_key.clone().add_path_info(path_info.clone());
                    let created = if stateful && property_name_tree_created {
                        insert_new_empty_tree(
                            path_key_info,
                            value_tree_type,
                            storage_flags,
                            batch_operations,
                        )?;
                        true
                    } else {
                        self.batch_insert_empty_tree_if_not_exists(
                            path_key_info,
                            value_tree_type,
                            storage_flags,
                            value_apply_type,
                            transaction,
                            previous_batch_operations,
                            batch_operations,
                            drive_version,
                        )?
                    };
                    let mut value_path_info = path_info.clone();
                    value_path_info.push(value_key.clone())?;

                    if let Some(estimated_costs_only_with_layer_info) =
                        estimated_costs_only_with_layer_info
                    {
                        // The value tree's own layer: its children are the `0`
                        // member bucket plus any continuation property-name trees.
                        estimated_costs_only_with_layer_info.insert(
                            value_path_info.clone().convert_to_key_info_path(),
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
                    frontier.push((value_path_info, created));
                }
            }

            parent_value_tree_type = value_tree_type;
            parent_counts_continuations = sub_level.is_ranked_chain_level();
        }

        if index.properties.is_empty() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a preallocated index must have properties",
            )));
        }

        // The terminal `0` member bucket of each path — mirror of the
        // tree-creation half of `add_index_only_terminal_item_operations`
        // (the member entry itself is each entry insert's own write).
        let level_info = terminal_level.has_index_with_type().ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "a preallocated index must terminate at its last property",
            ),
        ))?;
        let member_tree_type = terminal_member_tree_type(level_info);
        let member_apply_type = if stateful {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                // The `0` tree's parent is the index's value tree — same
                // aggregate-aware claim the entry-insert terminal makes.
                in_tree_type: terminal_value_tree_type(level_info),
                tree_type: member_tree_type,
                flags_len,
            }
        };
        // Same per-entry padding (and sum-item worst case) the entry-insert
        // terminal claims for this layer — see
        // `add_index_only_terminal_item_operations`.
        let member_layer_sizes = if stateful {
            None
        } else {
            let estimated_item_value_size =
                index_only_item_estimated_value_size(referring_type, platform_version)?;
            let member_key_max_size = match level_info.terminal.as_deref() {
                Some(terminal) => {
                    index_only_terminal_max_key_size(referring_type, terminal, platform_version)?
                }
                None => DEFAULT_HASH_SIZE_U8,
            };
            let estimated_value_size = if level_info.summable.is_some() {
                estimated_item_value_size + 10
            } else {
                estimated_item_value_size
            };
            Some(AllItems(
                member_key_max_size,
                estimated_value_size,
                storage_flags.map(|s| s.serialized_size()),
            ))
        };
        for (mut path_info, value_tree_created) in frontier {
            let path_key_info = KeyRef(&[0]).add_path_info(path_info.clone());
            if stateful && value_tree_created {
                insert_new_empty_tree(
                    path_key_info,
                    member_tree_type,
                    storage_flags,
                    batch_operations,
                )?;
            } else {
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
            }

            if let (Some(estimated_costs_only_with_layer_info), Some(member_layer_sizes)) = (
                estimated_costs_only_with_layer_info.as_mut(),
                member_layer_sizes,
            ) {
                path_info.push(Key(vec![0]))?;
                estimated_costs_only_with_layer_info.insert(
                    path_info.convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        tree_type: member_tree_type,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: member_layer_sizes,
                    },
                );
            }
        }

        Ok(())
    }
}

/// The one key of a level a binding fixes to a single value
/// (`key_source`): the referenced document's `$id` at the referring property,
/// otherwise the value of the referenced property the `where` binds, keyed as
/// the referring type keys `property_name`. `None` when no entry can ever
/// agree with the document, so nothing is preallocated: the bound value is
/// absent, null or wider than the referring property holds. Without a
/// document the key is sized as the referring property, the bound every key
/// an entry writes at this level has.
fn preallocated_single_key<'d>(
    document_and_contract_info: &'d DocumentAndContractInfo<'_>,
    referring_type: DocumentTypeRef,
    property_name: &str,
    sub_level: &IndexLevel,
    key_source: PreallocatedKeySource,
    event_id: [u8; 32],
    platform_version: &PlatformVersion,
) -> Result<Option<DriveKeyInfo<'d>>, Error> {
    let target_document_type = document_and_contract_info.document_type;
    let document_info = &document_and_contract_info.owned_document_info.document_info;
    // `$id` for the referring property; the agreed referenced
    // property otherwise — validated at contract registration to
    // share one value kind with the referring side, and keyed below
    // as the referring type keys it, so the bytes match what an
    // entry insert would write.
    let (source_property, source_type) = match key_source {
        PreallocatedKeySource::ReferencedDocumentId => ("$id", target_document_type),
        PreallocatedKeySource::ReferencedDocumentListElements(_) => {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a list key source resolves through \
                 referenced_list_tree_keys_for_referring_property, not as a single key",
            )))
        }
        PreallocatedKeySource::ReferencedDocumentProperty(referenced) => {
            match document_info.get_borrowed_document() {
                None => (property_name, referring_type),
                Some(document) => {
                    let referring_property_type = &referring_type
                        .flattened_properties()
                        .get(property_name)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "a preallocated index's property must be a property of \
                             its document type",
                        )))?
                        .property_type;
                    if !bound_value_fits_referring_property(
                        document,
                        referenced,
                        referring_property_type,
                        platform_version,
                    )? {
                        // No entry can ever agree with a value wider
                        // than the referring property holds, and it
                        // may not fit a tree key at all. Nothing to
                        // preallocate.
                        return Ok(None);
                    }
                    (referenced, target_document_type)
                }
            }
        }
    };
    let Some(value_key) = document_info.get_raw_for_document_type(
        source_property,
        source_type,
        document_and_contract_info.owned_document_info.owner_id,
        Some((sub_level, event_id)),
        platform_version,
    )?
    else {
        // The referenced document does not carry the bound property
        // (it is optional there): no entry can ever agree with it,
        // and if that changes on an update the entry-insert fallback
        // creates the trees. Nothing to preallocate.
        return Ok(None);
    };
    if value_key.is_empty() {
        // A null value takes the null index layout on the entry
        // side; leave that (never-preallocatable) shape entirely to
        // the fallback.
        return Ok(None);
    }
    Ok(Some(match (key_source, value_key) {
        (PreallocatedKeySource::ReferencedDocumentProperty(referenced), Key(key))
            if document_info.get_borrowed_document().is_some() =>
        {
            Key(referenced_tree_key_for_referring_property(
                key,
                target_document_type,
                referenced,
                referring_type,
                property_name,
                platform_version,
            )?)
        }
        (_, value_key) => value_key,
    }))
}

/// Queues the plain empty tree at `path_key_info` (no wrapper, no ranking)
/// without reading whether it exists, for a stateful walk under a tree it has
/// just created, which nothing can hold yet.
fn insert_new_empty_tree(
    path_key_info: PathKeyInfo<0>,
    tree_type: TreeType,
    storage_flags: Option<&StorageFlags>,
    batch_operations: &mut Vec<LowLevelDriveOperation>,
) -> Result<(), Error> {
    let (path, key) = match path_key_info {
        PathKeyInfo::PathKey((path, key)) => (path, key),
        PathKeyInfo::PathKeyRef((path, key)) => (path, key.to_vec()),
        _ => {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a stateful preallocation walks known paths",
            )))
        }
    };
    batch_operations.push(tree_type.empty_tree_operation_for_known_path_key(
        path,
        key,
        storage_flags,
    )?);
    Ok(())
}
