use crate::drive::constants::STORAGE_FLAGS_SIZE;
use crate::drive::document::index_level_tree_types::terminal_member_tree_type;
use crate::drive::document::{
    document_reference_size, make_document_reference, make_document_reference_with_sum_item,
    read_document_sum_contribution,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::util::object_size_info::DocumentInfo::{
    DocumentAndSerialization, DocumentEstimatedAverageSize, DocumentOwnedInfo,
    DocumentRefAndSerialization, DocumentRefInfo,
};
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::util::object_size_info::KeyElementInfo::{KeyElement, KeyUnknownElementSize};
use crate::util::object_size_info::{
    DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo, PathKeyElementInfo,
};
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::data_contract::document_type::IndexLevelTypeInfo;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::Document;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::{AllItems, AllReference};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

/// Which `Option<&StorageFlags>` the terminal reference element is built
/// with: v0 reads the document info's own flags, v1 the walker-passed ones.
#[derive(Clone, Copy)]
pub(super) enum TerminalReferenceFlagsSource {
    DocumentInfo,
    Walker,
}

impl TerminalReferenceFlagsSource {
    fn select<'a>(
        self,
        document_info: Option<&'a StorageFlags>,
        walker: Option<&'a StorageFlags>,
    ) -> Option<&'a StorageFlags> {
        match self {
            Self::DocumentInfo => document_info,
            Self::Walker => walker,
        }
    }
}

impl Drive {
    /// Adds the terminal reference.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_reference_for_index_level_for_contract_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        // See the wrapper's docstring for why this is a borrow now.
        index_type: &IndexLevelTypeInfo,
        any_fields_null: bool,
        all_fields_null: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.add_reference_for_index_level_for_contract_operations_inner(
            document_and_contract_info,
            index_path_info,
            index_type,
            any_fields_null,
            all_fields_null,
            previous_batch_operations,
            storage_flags,
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            platform_version,
            TerminalReferenceFlagsSource::DocumentInfo,
        )
    }

    /// Shared body of the v0 and v1 terminal-reference insert; the versions
    /// differ only in `terminal_flags_source`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_reference_for_index_level_for_contract_operations_inner(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        mut index_path_info: PathInfo<0>,
        // See the wrapper's docstring for why this is a borrow now.
        index_type: &IndexLevelTypeInfo,
        any_fields_null: bool,
        all_fields_null: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
        terminal_flags_source: TerminalReferenceFlagsSource,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;

        if all_fields_null && !index_type.should_insert_with_all_null {
            return Ok(());
        }

        // indexOnly terminal: the member key is the terminal property's
        // value and the element is an empty `Item` — there is no
        // primary-storage row to reference. `terminal` can only be `Some`
        // on a PV14+ indexOnly contract (the grammar rejects the keyword
        // below meta-schema v3), so this branch is unreachable for every
        // historical document — the same in-place gating the count and sum
        // flags in this function already rely on.
        if let Some(terminal_property) = index_type.terminal.as_deref() {
            return self.add_index_only_terminal_item_operations(
                document_and_contract_info,
                index_path_info,
                index_type,
                terminal_property,
                previous_batch_operations,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                platform_version,
            );
        }

        // The terminal reference's tree type is driven by the
        // composition of the index's countability AND summability,
        // per-axis (grovedb PR 670's expanded TreeType set
        // distinguishes provable from root-only on each axis
        // independently):
        //
        // - count provable + sum root → `ProvableCountSumTree`
        //   (existing variant: per-node count, root-only sum)
        // - count root + sum provable → `ProvableCountProvableSumTree`
        //   (no dedicated "count-root + sum-provable" variant exists;
        //   upgrades count to per-node too)
        // - count provable + sum provable →
        //   `ProvableCountProvableSumTree` (PR 670 newcomer: both
        //   per-node)
        //
        // Same dispatch shape as the primary-key tree dispatcher's v1
        // arm in `primary_key_tree_type.rs` — see
        // `terminal_member_tree_type` for the full table (shared with
        // the delete side and the indexOnly terminal branches). The
        // `IndexLevelTypeInfo`'s `summable` carries the property name
        // the reference's sum-item will contribute (read below to
        // construct the `Element::ReferenceWithSumItem` that replaces a
        // plain `Element::Reference` under summable indexes).
        let reference_tree_type = terminal_member_tree_type(index_type);

        // Element-shape selector. Under a summable index path the
        // reference element MUST be
        // `Element::ReferenceWithSumItem(reference_path, amount_i64,
        // flags)` (grovedb PR 670) rather than a plain
        // `Element::Reference` — only `ReferenceWithSumItem`
        // contributes a sum to the ancestor sum trees while still
        // dereferencing to the document body in primary storage
        // (so document iteration via index walks keeps working
        // identically to the count side). Read the sum contribution
        // once per insert from the document's `summable.unwrap()`
        // property and freeze it into the element. On delete, grovedb
        // pulls the same sum value off the stored element and
        // propagates the subtraction up the merk path — no need to
        // re-read the source document on the way down.
        let sum_property_name: Option<&str> = index_type.summable.as_deref();
        let make_terminal_ref =
            |document: &Document, storage_flags: Option<&StorageFlags>| -> Result<Element, Error> {
                match sum_property_name {
                    Some(prop_name) => {
                        // DPP validator guarantees the property is in
                        // `required` and is an integer type, so this
                        // conversion is safe — propagated as
                        // `CorruptedCodeExecution` if it ever fails.
                        let sum_value = read_document_sum_contribution(document, prop_name)?;
                        Ok(make_document_reference_with_sum_item(
                            document,
                            document_and_contract_info.document_type,
                            sum_value,
                            storage_flags,
                        ))
                    }
                    None => Ok(make_document_reference(
                        document,
                        document_and_contract_info.document_type,
                        storage_flags,
                    )),
                }
            };
        // unique indexes will be stored under key "0"
        // non-unique indices should have a tree at key "0" that has all elements based off of primary key
        if !index_type.index_type.is_unique() || any_fields_null {
            // Tree generation, this happens for both non unique indexes, unique indexes with a null inside
            // a member of the path
            let key_path_info = KeyRef(&[0]);

            let path_key_info = key_path_info.add_path_info(index_path_info.clone());

            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: TreeType::NormalTree,
                    tree_type: reference_tree_type,
                    flags_len: storage_flags
                        .map(|s| s.serialized_size())
                        .unwrap_or_default(),
                }
            };

            // Here we are inserting an empty tree that will have a subtree of all other index properties
            // It is basically the 0
            // Underneath we will have all elements if non unique index, or all identity contenders if
            // a contested resource index
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info,
                reference_tree_type,
                *storage_flags,
                apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                drive_version,
            )?;

            index_path_info.push(Key(vec![0]))?;
            // This is the simpler situation
            // Under each tree we have all the references

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                // On this level we will have a 0 and all the top index paths
                estimated_costs_only_with_layer_info.insert(
                    index_path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        tree_type: reference_tree_type,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllReference(
                            DEFAULT_HASH_SIZE_U8,
                            document_reference_size(document_and_contract_info.document_type),
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            let key_element_info =
                match &document_and_contract_info.owned_document_info.document_info {
                    DocumentRefAndSerialization((document, _, document_flags))
                    | DocumentRefInfo((document, document_flags)) => {
                        let document_reference = make_terminal_ref(
                            document,
                            terminal_flags_source.select(
                                document_flags.as_ref().map(|flags| flags.as_ref()),
                                *storage_flags,
                            ),
                        )?;
                        KeyElement((document.id_ref().as_slice(), document_reference))
                    }
                    DocumentOwnedInfo((document, document_flags))
                    | DocumentAndSerialization((document, _, document_flags)) => {
                        let document_reference = make_terminal_ref(
                            document,
                            terminal_flags_source.select(
                                document_flags.as_ref().map(|flags| flags.as_ref()),
                                *storage_flags,
                            ),
                        )?;
                        KeyElement((document.id_ref().as_slice(), document_reference))
                    }
                    DocumentEstimatedAverageSize(max_size) => KeyUnknownElementSize((
                        KeyInfo::MaxKeySize {
                            unique_id: document_and_contract_info
                                .document_type
                                .unique_id_for_storage()
                                .to_vec(),
                            max_size: DEFAULT_HASH_SIZE_U8,
                        },
                        // Match the sum-bearing variant the live path
                        // would have written: `make_document_reference_with_sum_item`
                        // emits `Element::ReferenceWithSumItem` when
                        // `sum_property_name.is_some()`. The sum-aware helper
                        // reserves 10 worst-case bytes for the i64 sum_value.
                        // Unconditional switch: this entire flow is v12+
                        // gated (no v11 consensus baseline for sum-bearing
                        // index refs).
                        if sum_property_name.is_some() {
                            Element::required_reference_with_sum_item_space(
                                *max_size,
                                STORAGE_FLAGS_SIZE,
                                &drive_version.grove_version,
                            )?
                        } else {
                            Element::required_item_space(
                                *max_size,
                                STORAGE_FLAGS_SIZE,
                                &drive_version.grove_version,
                            )?
                        },
                    )),
                };

            let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
                index_path_info,
                key_element_info,
            )?;

            // here we should return an error if the element already exists
            self.batch_insert(path_key_element_info, batch_operations, drive_version)?;
        } else {
            let key_element_info =
                match &document_and_contract_info.owned_document_info.document_info {
                    DocumentRefAndSerialization((document, _, document_flags))
                    | DocumentRefInfo((document, document_flags)) => {
                        let document_reference = make_terminal_ref(
                            document,
                            terminal_flags_source.select(
                                document_flags.as_ref().map(|flags| flags.as_ref()),
                                *storage_flags,
                            ),
                        )?;
                        KeyElement((&[0], document_reference))
                    }
                    DocumentOwnedInfo((document, document_flags))
                    | DocumentAndSerialization((document, _, document_flags)) => {
                        let document_reference = make_terminal_ref(
                            document,
                            terminal_flags_source.select(
                                document_flags.as_ref().map(|flags| flags.as_ref()),
                                *storage_flags,
                            ),
                        )?;
                        KeyElement((&[0], document_reference))
                    }
                    DocumentEstimatedAverageSize(estimated_size) => KeyUnknownElementSize((
                        KeyInfo::MaxKeySize {
                            unique_id: document_and_contract_info
                                .document_type
                                .unique_id_for_storage()
                                .to_vec(),
                            max_size: 1,
                        },
                        // Parallel to the non-unique branch above: unique
                        // indexes with `summable: Some(_)` still write a
                        // `ReferenceWithSumItem` at the terminal `[0]` slot
                        // when there's any non-null entry (the unique-no-op
                        // caveat applies only to all-non-null exact matches,
                        // see book/document-sum-trees.md). The estimated
                        // worst-case treats the sum-bearing variant.
                        if sum_property_name.is_some() {
                            Element::required_reference_with_sum_item_space(
                                *estimated_size,
                                STORAGE_FLAGS_SIZE,
                                &drive_version.grove_version,
                            )?
                        } else {
                            Element::required_item_space(
                                *estimated_size,
                                STORAGE_FLAGS_SIZE,
                                &drive_version.grove_version,
                            )?
                        },
                    )),
                };

            let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
                index_path_info,
                key_element_info,
            )?;

            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertApplyType::StatefulBatchInsert
            } else {
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_type: reference_tree_type,
                    target: QueryTargetValue(
                        document_reference_size(document_and_contract_info.document_type)
                            + storage_flags
                                .map(|s| s.serialized_size())
                                .unwrap_or_default(),
                    ),
                }
            };

            // here we should return an error if the element already exists
            let inserted = self.batch_insert_if_not_exists(
                path_key_element_info,
                apply_type,
                transaction,
                batch_operations,
                drive_version,
            )?;
            if !inserted {
                return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                    "reference already exists".to_string(),
                )));
            }
        }
        Ok(())
    }

    /// The indexOnly terminal: writes `[…index path, 0, <terminal value>] →
    /// Item(commitment, flags)` — the member key is the terminal property's
    /// value (`$ownerId` or a refersTo-typed identifier) sitting exactly
    /// where a normal non-unique index keys by document id, and the element
    /// payload is the row commitment because the entry IS the row. Storage
    /// flags ride the element flags for epoch/owner refunds, exactly as on
    /// references.
    ///
    /// Existence check semantics: the entry is inserted with
    /// `if_not_exists`, and an existing entry is an error. ABCI state
    /// validation probes every index's entry before the batch applies, so
    /// reaching this error at apply time means validation was bypassed —
    /// the same backstop role the unique-index "reference already exists"
    /// above plays.
    ///
    /// Under a summable index the element is
    /// `ItemWithSumItem(commitment, amount, flags)` instead — the same
    /// payload, plus the summed property's value read off the document, so
    /// the entry contributes to the ancestor sum trees exactly as a
    /// `ReferenceWithSumItem` does for stored types. On delete, grovedb
    /// reads the amount off the stored element and propagates the
    /// subtraction — the delete path needs no sum-specific logic. The
    /// amount is one of the document's properties, so it is already covered
    /// by the row commitment: a delete carrying a falsified amount fails
    /// the commitment probe before anything is removed.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_index_only_terminal_item_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        mut index_path_info: PathInfo<0>,
        index_type: &IndexLevelTypeInfo,
        terminal_property: &str,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;

        let member_tree_type = terminal_member_tree_type(index_type);
        let sum_property_name: Option<&str> = index_type.summable.as_deref();

        // The `0` storage-marker tree, byte-identical in position to the
        // non-unique layout above.
        let key_path_info = KeyRef(&[0]);
        let path_key_info = key_path_info.add_path_info(index_path_info.clone());

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                // The `0` tree's parent is the index's value tree, which
                // aggregates whenever the index counts or sums — claiming
                // `NormalTree` here under-estimates the parent's per-child
                // aggregate bytes, and a time-range index's bucket fan-out
                // multiplies the gap past the item-size padding (the
                // `estimated_fees_upper_bound_actual_fees` e2e tests pin
                // the invariant).
                in_tree_type:
                    crate::drive::document::index_level_tree_types::terminal_value_tree_type(
                        index_type,
                    ),
                tree_type: member_tree_type,
                flags_len: storage_flags
                    .map(|s| s.serialized_size())
                    .unwrap_or_default(),
            }
        };

        self.batch_insert_empty_tree_if_not_exists(
            path_key_info,
            member_tree_type,
            *storage_flags,
            apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;

        index_path_info.push(Key(vec![0]))?;

        // The payload is the 32-byte row commitment; the estimated per-item
        // value size is padded above it because the estimation layers
        // under-count each entry's chain (the serialized item envelope —
        // enum tag, length prefix, flags option — plus the per-entry share
        // of parent-tree aggregate bytes), and estimation must UPPER-bound
        // the applied fee. The padding is per entry, so it scales with a
        // time-range index's bucket fan-out (one entry chain per bucket),
        // where the original 16-byte pad measurably under-ran — the
        // `estimated_fees_upper_bound_actual_fees` e2e tests (like / tip /
        // beat) pin the invariant across the plain, summable and bucketed
        // shapes.
        const INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE: u32 =
            crate::drive::document::INDEX_ONLY_ROW_COMMITMENT_SIZE + 32;

        // Sum-bearing entries additionally carry the i64 sum item in the
        // element envelope; 10 bytes is the worst case the sum-aware space
        // helpers reserve.
        let estimated_value_size = if sum_property_name.is_some() {
            INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE + 10
        } else {
            INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE
        };

        let item_space = if sum_property_name.is_some() {
            Element::required_item_with_sum_item_space(
                INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE,
                STORAGE_FLAGS_SIZE,
                &drive_version.grove_version,
            )?
        } else {
            Element::required_item_space(
                INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE,
                STORAGE_FLAGS_SIZE,
                &drive_version.grove_version,
            )?
        };

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
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

        // Member key: the terminal property's value — 32 bytes, since the
        // parser only admits `$ownerId` or identifier-typed refersTo
        // properties as terminals.
        let member_key: Option<Vec<u8>> = match document_and_contract_info
            .owned_document_info
            .document_info
            .get_borrowed_document_and_storage_flags()
        {
            Some((document, _)) => Some(
                document
                    .get_raw_for_document_type(
                        terminal_property,
                        document_and_contract_info.document_type,
                        document_and_contract_info.owned_document_info.owner_id,
                        platform_version,
                    )?
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                        "indexOnly terminal value must be present: the parser requires \
                         every indexOnly property (and $ownerId) to be set",
                    )))?,
            ),
            // Estimation-only document info carries no values.
            None => None,
        };

        // The applied element carries the 32-byte row commitment binding
        // this entry to the document's full value tuple (see
        // `index_only_row_commitment`); the dry run pads to the estimated
        // size, which keeps the estimate above the applied fee across the
        // indexed-tree layers' documented under-count (see
        // `estimated_sum_trees_for_value_tree_type`).
        let item_value = if estimated_costs_only_with_layer_info.is_some() {
            vec![0u8; INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE as usize]
        } else {
            let (document, _) = document_and_contract_info
                .owned_document_info
                .document_info
                .get_borrowed_document_and_storage_flags()
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "indexOnly terminal insert needs a document outside estimation mode",
                )))?;
            crate::drive::document::index_only_row_commitment(
                document,
                document_and_contract_info.document_type,
                platform_version,
            )?
            .to_vec()
        };

        let key_element_info = match &member_key {
            Some(member_key) => {
                let element_flags = StorageFlags::map_to_some_element_flags(*storage_flags);
                let item = match sum_property_name {
                    Some(prop_name) => {
                        let (document, _) = document_and_contract_info
                            .owned_document_info
                            .document_info
                            .get_borrowed_document_and_storage_flags()
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "indexOnly summable terminal insert needs a document to read \
                                 the sum contribution from",
                            )))?;
                        let sum_value = read_document_sum_contribution(document, prop_name)?;
                        Element::new_item_with_sum_item_with_flags(
                            item_value,
                            sum_value,
                            element_flags,
                        )
                    }
                    None => Element::new_item_with_flags(item_value, element_flags),
                };
                KeyElement((member_key.as_slice(), item))
            }
            None => KeyUnknownElementSize((
                KeyInfo::MaxKeySize {
                    unique_id: document_and_contract_info
                        .document_type
                        .unique_id_for_storage()
                        .to_vec(),
                    max_size: DEFAULT_HASH_SIZE_U8,
                },
                item_space,
            )),
        };

        let path_key_element_info =
            PathKeyElementInfo::from_path_info_and_key_element(index_path_info, key_element_info)?;

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertApplyType::StatefulBatchInsert
        } else {
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_type: member_tree_type,
                target: QueryTargetValue(
                    estimated_value_size
                        + storage_flags
                            .map(|s| s.serialized_size())
                            .unwrap_or_default(),
                ),
            }
        };

        let inserted = self.batch_insert_if_not_exists(
            path_key_element_info,
            apply_type,
            transaction,
            batch_operations,
            drive_version,
        )?;
        if !inserted {
            return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                "index-only entry already exists: state validation must reject a create \
                 whose entries collide before it reaches storage"
                    .to_string(),
            )));
        }

        Ok(())
    }
}
