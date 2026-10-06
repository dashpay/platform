//! The counter a `summableOffCountIndex` index keeps per group.
//!
//! A `summableOffCountIndex` index stores no entry per document. At the value
//! position of its last property, where another index would grow a value
//! tree, a `0` bucket and one member entry per document, it keeps one
//! `Element::SumItem` holding the number of its source index's entries in
//! that group. The item adds its entries to the sum of every tree above it, so
//! every level above reads entries as the sum (likes); in count-and-sum trees
//! (an average ranking or `rangeCountable` adds counts) it also counts one
//! group, so those levels read groups as the count (posts) and an average
//! reads entries per group (likes per post).
//!
//! A create moves the counter up by one and a delete down by one, each in the
//! same batch as the document's other entries. A create is refused before it
//! gets here when an entry of the source already exists, and a delete only
//! once the entries of the indexes that keep them matched its row commitment,
//! so the counter always equals the source group's entry count.
//!
//! The counter is rewritten in place: a `SumItem` is charged a fixed size
//! whatever its value, so a rewrite stores nothing new and the rewritten item
//! keeps the storage flags of whoever first paid for it. A preallocated index
//! creates every counter at zero with the referenced document and keeps it at
//! zero; otherwise the first create of a group inserts it and the delete of
//! its last document removes it, pruning the trees it leaves empty.

use crate::drive::constants::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::grove_operations::pending_grove_operations::pending_grove_operations;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{
    BatchDeleteUpTreeApplyType, BatchInsertApplyType, DirectQueryType,
};
use crate::util::object_size_info::KeyElementInfo::{KeyElement, KeyElementSize};
use crate::util::object_size_info::{DriveKeyInfo, KeyElementInfo, PathInfo, PathKeyElementInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::AllItems;
use grovedb::{
    Element, ElementFlags, EstimatedLayerInformation, GroveDb, MaybeTree, TransactionArg, TreeType,
};
use grovedb_merk::tree_type::SUM_ITEM_COST_SIZE;
use grovedb_storage::worst_case_costs::WorstKeyLength;
use std::collections::HashMap;

/// How a write moves a `summableOffCountIndex` index's counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterChange {
    /// A create: one more document in the group.
    Increment {
        /// Whether this write created the tree the counter sits in, so the
        /// counter cannot exist yet and the stateful write skips its read
        /// (an estimate still prices it).
        parent_created: bool,
    },
    /// A delete: one document fewer. A preallocated index keeps the counter
    /// at zero; any other removes it with its last document, pruning the trees
    /// above it that it leaves empty, up to the document type's level
    /// (`CONTRACT_DOCUMENTS_PATH_HEIGHT`), as a drained member bucket's are.
    Decrement {
        /// Whether the index is preallocated.
        keep_at_zero: bool,
    },
}

/// Registers the estimated layer a `summableOffCountIndex` index's counters
/// sit in, at `counter_path`: the tree of its last property, of type
/// `counter_tree_type`, whose children are `SumItem`s keyed by that
/// property's values, `estimated_key_size` bytes at most. The insert and
/// delete walkers and preallocation all register it here.
pub(crate) fn insert_summable_off_count_counter_layer(
    estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    counter_path: KeyInfoPath,
    counter_tree_type: TreeType,
    estimated_key_size: u16,
    storage_flags: Option<&StorageFlags>,
) -> Result<(), Error> {
    let max_key_size = u8::try_from(estimated_key_size).map_err(|_| {
        Error::Fee(FeeError::Overflow(
            "document field is too big for being an index",
        ))
    })?;
    estimated_costs_only_with_layer_info.insert(
        counter_path,
        EstimatedLayerInformation {
            tree_type: counter_tree_type,
            estimated_layer_count: PotentiallyAtMaxElements,
            estimated_layer_sizes: AllItems(
                max_key_size,
                SUM_ITEM_COST_SIZE,
                storage_flags.map(|flags| flags.serialized_size()),
            ),
        },
    );
    Ok(())
}

/// The counter at zero, keyed by `counter_key`, as a preallocation inserts it.
fn zero_counter_key_element<'a>(
    counter_key: &'a DriveKeyInfo<'a>,
    element_flags: Option<ElementFlags>,
) -> KeyElementInfo<'a> {
    let counter = Element::new_sum_item_with_flags(0, element_flags);
    match counter_key {
        DriveKeyInfo::Key(key) => KeyElement((key.as_slice(), counter)),
        DriveKeyInfo::KeyRef(key) => KeyElement((key, counter)),
        DriveKeyInfo::KeySize(key_info) => KeyElementSize((key_info.clone(), counter)),
    }
}

impl Drive {
    /// Adds the operations that move one group's counter of a
    /// `summableOffCountIndex` index: `counter_path_info` is the path of the tree of the index's last
    /// property, `counter_key` the group's value of it, and
    /// `counter_tree_type` that tree's type.
    ///
    /// A stateful call reads the counter and writes it back moved; an
    /// estimation call (`estimated_costs_only_with_layer_info` set) registers
    /// the counter's layer, its keys sized by `estimated_key_size` (called
    /// only then), reads nothing and prices the read and the write the change
    /// makes: an insert for a create, a rewrite or the removal for a delete. A counter is
    /// written at most once per batch because a documents batch carries one
    /// transition (`max_transitions_in_documents_batch`), not because of the
    /// source: a source keyed by more than its owner holds several entries of
    /// one owner in one group. Each document's conversion reads the counter
    /// from committed state, so two documents moving one counter in one batch
    /// would both write the same next value. The guard against that is the
    /// batch methods' `Drive::refuse_repeated_counter_moves`, which refuses a
    /// batch moving one type's counters for more than one document (an
    /// insert of a referenced document that preallocates them included),
    /// estimation included; raising the cap needs the counter moves folded
    /// across documents first (see that limit). The check below is defence
    /// for a direct Drive caller: it refuses, rather than folds, a second
    /// write of the counter that this conversion's operations, or the
    /// `previous_batch_operations` a caller hands in (a multi-document
    /// operation passes the earlier documents' operations), already hold.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_summable_off_count_counter_operations(
        &self,
        counter_path_info: PathInfo<0>,
        counter_key: DriveKeyInfo,
        counter_tree_type: TreeType,
        change: CounterChange,
        storage_flags: Option<&StorageFlags>,
        estimated_key_size: impl FnOnce() -> Result<u16, Error>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        previous_batch_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        let flags_len = storage_flags.map_or(0, |flags| flags.serialized_size());
        let element_flags = StorageFlags::map_to_some_element_flags(storage_flags);

        if let Some(layers) = estimated_costs_only_with_layer_info.as_mut() {
            insert_summable_off_count_counter_layer(
                layers,
                counter_path_info.clone().convert_to_key_info_path(),
                counter_tree_type,
                estimated_key_size()?,
                storage_flags,
            )?;
            let key_info_path = counter_path_info.convert_to_key_info_path();
            let key_info = counter_key.to_key_info();
            // The read of the counter, priced at its fixed size. The removal
            // below prices its own read of the counter, so only a write that
            // keeps the counter pushes this one.
            let counter_read = || {
                GroveDb::average_case_for_get_raw(
                    &key_info_path,
                    &key_info,
                    SUM_ITEM_COST_SIZE + flags_len,
                    counter_tree_type,
                    &drive_version.grove_version,
                )
                .map(CalculatedCostOperation)
            };
            match change {
                CounterChange::Increment { .. } => {
                    batch_operations.push(counter_read()?);
                    batch_operations.push(
                        LowLevelDriveOperation::insert_for_estimated_path_key_element(
                            key_info_path,
                            key_info,
                            Element::new_sum_item_with_flags(1, element_flags),
                        ),
                    );
                }
                CounterChange::Decrement {
                    keep_at_zero: true, ..
                } => {
                    batch_operations.push(counter_read()?);
                    batch_operations.push(
                        LowLevelDriveOperation::replace_for_estimated_path_key_element(
                            key_info_path,
                            key_info,
                            Element::new_sum_item_with_flags(0, element_flags),
                        ),
                    );
                }
                // Priced as the removal also when the stateful path only
                // rewrites the counter (the group keeps other entries): the
                // removal costs at least the rewrite
                // (`should_upper_bound_an_unlike_with_its_dry_run`).
                CounterChange::Decrement {
                    keep_at_zero: false,
                } => {
                    let key_size = key_info.max_length();
                    let apply_type = Self::stateless_delete_of_non_tree_for_costs(
                        AllItems(key_size, SUM_ITEM_COST_SIZE, Some(flags_len)),
                        &key_info_path,
                        Some(MaybeTree::NotTree),
                        estimated_costs_only_with_layer_info,
                        platform_version,
                    )?;
                    self.batch_delete_up_tree_while_empty(
                        key_info_path,
                        key_info.as_slice(),
                        Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                        apply_type,
                        transaction,
                        previous_batch_operations,
                        batch_operations,
                        drive_version,
                    )?;
                }
            }
            return Ok(());
        }

        let PathInfo::PathAsVec(counter_path) = counter_path_info else {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a stateful counter write walks a known index path",
            )));
        };
        let counter_key = match counter_key {
            DriveKeyInfo::Key(key) => key,
            DriveKeyInfo::KeyRef(key) => key.to_vec(),
            DriveKeyInfo::KeySize(_) => {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a stateful counter write knows the group's value",
                )))
            }
        };
        let counter_key_info_path = KeyInfoPath::from_known_owned_path(counter_path.clone());
        let counter_key_info = KeyInfo::KnownKey(counter_key.clone());
        let already_written = previous_batch_operations
            .as_deref()
            .is_some_and(|operations| {
                counter_queued(operations, &counter_key_info_path, &counter_key_info)
            })
            || counter_queued(batch_operations, &counter_key_info_path, &counter_key_info);
        if already_written {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a summableOffCountIndex index's counter is written twice in one batch",
            )));
        }

        let existing = if matches!(
            change,
            CounterChange::Increment {
                parent_created: true
            }
        ) {
            None
        } else {
            self.grove_get_raw_optional(
                counter_path.as_slice().into(),
                counter_key.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                batch_operations,
                drive_version,
            )?
        };
        let (count, stored_flags) =
            match existing {
                None => (None, None),
                Some(Element::SumItem(count, flags)) => (Some(count), flags),
                Some(_) => return Err(Error::Drive(DriveError::CorruptedElementType(
                    "a summableOffCountIndex index's group holds something other than a sum item",
                ))),
            };
        match change {
            CounterChange::Increment { .. } => {
                // A rewrite keeps the flags of whoever first paid for the
                // item; the first create of a group pays for it.
                let (count, flags) = match count {
                    Some(count) => (
                        count.checked_add(1).ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "a summableOffCountIndex index's counter overflowed".to_string(),
                            ))
                        })?,
                        stored_flags,
                    ),
                    None => (1, element_flags),
                };
                batch_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    counter_path,
                    counter_key,
                    Element::new_sum_item_with_flags(count, flags),
                ));
            }
            CounterChange::Decrement { keep_at_zero } => {
                let count = count.filter(|count| *count > 0).ok_or_else(|| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "a delete reached a summableOffCountIndex index whose group counts no document"
                            .to_string(),
                    ))
                })?;
                if count == 1 && !keep_at_zero {
                    self.batch_delete_up_tree_while_empty(
                        KeyInfoPath::from_known_owned_path(counter_path),
                        counter_key.as_slice(),
                        Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                        BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                        },
                        transaction,
                        previous_batch_operations,
                        batch_operations,
                        drive_version,
                    )?;
                } else {
                    batch_operations.push(
                        LowLevelDriveOperation::replace_for_known_path_key_element(
                            counter_path,
                            counter_key,
                            Element::new_sum_item_with_flags(count - 1, stored_flags),
                        ),
                    );
                }
            }
        }
        Ok(())
    }
    /// Adds the operations that preallocate one group's counter of a
    /// `summableOffCountIndex` index at zero, as the referenced document's
    /// insert creates it: `counter_path_info`, `counter_key` and
    /// `counter_tree_type` as for
    /// [`Self::add_summable_off_count_counter_operations`]. It inserts the
    /// counter when absent and keeps it as it stands when a
    /// `moderatedDocument` restore finds it kept, never moving it.
    /// `cannot_exist` skips the stateful existence read: under a tree this
    /// walk just created, or through a `permanentDocument` reference, whose
    /// key is the inserted document's `$id`, inserted once and never
    /// restored. An estimation call registers the counter's layer and prices
    /// the existence read either way, so it bounds every stateful write.
    /// `previous_batch_operations` are the operations the batch already
    /// queued, the document's earlier bindings' among them: two bindings of
    /// one index can resolve the same counter (their `where` agreements
    /// meeting at one path), and a counter an earlier binding queued is
    /// neither read nor queued again, estimated or not.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_summable_off_count_zero_counter_operations(
        &self,
        counter_path_info: PathInfo<0>,
        counter_key: DriveKeyInfo,
        counter_tree_type: TreeType,
        cannot_exist: bool,
        storage_flags: Option<&StorageFlags>,
        previous_batch_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_key_size: impl FnOnce() -> Result<u16, Error>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        if previous_batch_operations
            .as_deref()
            .is_some_and(|operations| {
                counter_queued(
                    operations,
                    &counter_path_info.clone().convert_to_key_info_path(),
                    &counter_key.to_key_info(),
                )
            })
        {
            return Ok(());
        }
        let element_flags = StorageFlags::map_to_some_element_flags(storage_flags);
        let estimating = estimated_costs_only_with_layer_info.is_some();
        if let Some(layers) = estimated_costs_only_with_layer_info.as_mut() {
            insert_summable_off_count_counter_layer(
                layers,
                counter_path_info.clone().convert_to_key_info_path(),
                counter_tree_type,
                estimated_key_size()?,
                storage_flags,
            )?;
        }
        let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
            counter_path_info,
            zero_counter_key_element(&counter_key, element_flags),
        )?;
        if estimating {
            let flags_len = storage_flags.map_or(0, |flags| flags.serialized_size());
            self.batch_insert_if_not_exists(
                path_key_element_info,
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_type: counter_tree_type,
                    target: QueryTargetValue(SUM_ITEM_COST_SIZE + flags_len),
                },
                transaction,
                batch_operations,
                drive_version,
            )?;
        } else if cannot_exist {
            self.batch_insert(path_key_element_info, batch_operations, drive_version)?;
        } else {
            self.batch_insert_if_not_exists(
                path_key_element_info,
                BatchInsertApplyType::StatefulBatchInsert,
                transaction,
                batch_operations,
                drive_version,
            )?;
        }
        Ok(())
    }
}

/// Whether `operations` already queue a write of the counter at `path` and
/// `key`. The one test both counter writes read: a create's move refuses a
/// second write of one counter in a batch, and a preallocation skips a zero
/// counter an earlier binding queued.
fn counter_queued(
    operations: &[LowLevelDriveOperation],
    path: &KeyInfoPath,
    key: &KeyInfo,
) -> bool {
    pending_grove_operations(operations)
        .any(|operation| operation.path == *path && operation.key.as_ref() == Some(key))
}
