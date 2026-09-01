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
use crate::util::object_size_info::{DocumentAndContractInfo, PathInfo, PathKeyElementInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::data_contract::document_type::IndexLevelTypeInfo;
use dpp::document::Document;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::AllReference;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds the terminal reference.
    ///
    /// v1: the terminal reference element takes the storage flags the walker
    /// passed down, not the document info's own — v0 read the latter, which
    /// diverges exactly when a walker level decides its elements carry no
    /// flags (immutable doctypes historically, TTL'd (ephemeral) sub-levels
    /// now). Ephemeral references must be flagless or their removal turns
    /// sectioned (refundable), breaking the TTL no-refunds invariant.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_reference_for_index_level_for_contract_operations_v1(
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

            let key_element_info = match &document_and_contract_info
                .owned_document_info
                .document_info
            {
                DocumentRefAndSerialization((document, _, _)) | DocumentRefInfo((document, _)) => {
                    let document_reference = make_terminal_ref(document, *storage_flags)?;
                    KeyElement((document.id_ref().as_slice(), document_reference))
                }
                DocumentOwnedInfo((document, _)) | DocumentAndSerialization((document, _, _)) => {
                    let document_reference = make_terminal_ref(document, *storage_flags)?;
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
            let key_element_info = match &document_and_contract_info
                .owned_document_info
                .document_info
            {
                DocumentRefAndSerialization((document, _, _)) | DocumentRefInfo((document, _)) => {
                    let document_reference = make_terminal_ref(document, *storage_flags)?;
                    KeyElement((&[0], document_reference))
                }
                DocumentOwnedInfo((document, _)) | DocumentAndSerialization((document, _, _)) => {
                    let document_reference = make_terminal_ref(document, *storage_flags)?;
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
}
