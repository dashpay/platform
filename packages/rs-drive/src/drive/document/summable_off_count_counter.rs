//! The counter a `summableOffCountIndex` index keeps per group.
//!
//! A `summableOffCountIndex` index stores no entry per document. At the value
//! position of its last property, where another index would grow a value
//! tree, a `0` bucket and one member entry per document, it keeps one
//! `Element::SumItem` holding the number of its source index's entries in
//! that group. In the count-and-sum trees it sits in, the item counts one
//! group and adds its entries to the sum, so every level above reads groups
//! as the count (posts) and entries as the sum (likes), and an average reads
//! entries per group (likes per post).
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

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::grove_operations::pending_grove_operations::pending_grove_operations;
use crate::util::grove_operations::{BatchDeleteUpTreeApplyType, DirectQueryType};
use crate::util::object_size_info::{DriveKeyInfo, PathInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::AllItems;
use grovedb::{Element, EstimatedLayerInformation, GroveDb, MaybeTree, TransactionArg, TreeType};
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
    /// above up to `stop_path_height`.
    Decrement {
        /// Whether the index is preallocated.
        keep_at_zero: bool,
        /// The path height the pruning climb stops at.
        stop_path_height: u16,
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

impl Drive {
    /// Adds the operations that move one group's counter of a
    /// `summableOffCountIndex` index: `counter_path_info` is the path of the tree of the index's last
    /// property, `counter_key` the group's value of it, and
    /// `counter_tree_type` that tree's type.
    ///
    /// A stateful call reads the counter and writes it back moved; an
    /// estimation call (`estimated_costs_only_with_layer_info` set) reads
    /// nothing and prices the read and the write the change makes: an insert
    /// for a create, a rewrite or the removal for a delete. A counter is
    /// written at most once per batch because a documents batch carries one
    /// transition (`max_transitions_in_documents_batch`), not because of the
    /// source: a source keyed by more than its owner holds several entries of
    /// one owner in one group. Each document is converted on its own
    /// (`previous_batch_operations` is empty across documents), so the check
    /// below catches only a second write within one conversion, refused
    /// rather than folded, and the batch methods refuse a batch writing two
    /// documents of a type keeping counters
    /// (`Drive::refuse_repeated_counter_moves`), estimation included; raising
    /// the cap needs the counter moves folded across documents first (see
    /// that limit).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_summable_off_count_counter_operations(
        &self,
        counter_path_info: PathInfo<0>,
        counter_key: DriveKeyInfo,
        counter_tree_type: TreeType,
        change: CounterChange,
        storage_flags: Option<&StorageFlags>,
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

        if estimated_costs_only_with_layer_info.is_some() {
            let key_info_path = counter_path_info.convert_to_key_info_path();
            let key_info = counter_key.to_key_info();
            // The read of the counter, priced at its fixed size.
            batch_operations.push(CalculatedCostOperation(GroveDb::average_case_for_get_raw(
                &key_info_path,
                &key_info,
                SUM_ITEM_COST_SIZE + flags_len,
                counter_tree_type,
                &drive_version.grove_version,
            )?));
            match change {
                CounterChange::Increment { .. } => {
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
                    stop_path_height,
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
                        Some(stop_path_height),
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
        let already_written = previous_batch_operations
            .as_deref()
            .into_iter()
            .flat_map(|operations| pending_grove_operations(operations))
            .chain(pending_grove_operations(batch_operations))
            .any(|operation| {
                operation
                    .key
                    .as_ref()
                    .is_some_and(|key| key.as_slice() == counter_key.as_slice())
                    && operation.path.to_path() == counter_path
            });
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
            CounterChange::Decrement {
                keep_at_zero,
                stop_path_height,
            } => {
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
                        Some(stop_path_height),
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
}
