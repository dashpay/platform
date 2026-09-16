use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees};
use grovedb::{EstimatedLayerInformation, MaybeTree, TransactionArg};

use dpp::data_contract::document_type::IndexLevelTypeInfo;
use dpp::data_contract::document_type::IndexType::{ContestedResourceIndex, NonUniqueIndex};
use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use crate::drive::constants::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::document::document_reference_size;
use crate::drive::document::index_level_tree_types::terminal_member_tree_type;
use crate::error::drive::DriveError;
use crate::util::storage_flags::StorageFlags;
use dpp::document::document_methods::DocumentMethodsV0;

use crate::drive::Drive;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo};

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::version::PlatformVersion;

impl Drive {
    /// Removes the terminal reference.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_reference_for_index_level_for_contract_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        // Borrow rather than owned — see the parallel change on the
        // insert path (`add_reference_for_index_level_for_contract_operations`)
        // for the rationale (`IndexLevelTypeInfo` dropped `Copy` when
        // `summable: Option<String>` was added in v3).
        index_type: &IndexLevelTypeInfo,
        any_fields_null: bool,
        all_fields_null: bool,
        storage_flags: &Option<&StorageFlags>,
        previous_batch_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        event_id: [u8; 32],
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if all_fields_null && !index_type.should_insert_with_all_null {
            return Ok(());
        }
        let mut key_info_path = index_path_info.convert_to_key_info_path();

        let document_type = document_and_contract_info.document_type;

        // indexOnly terminal: the member key is the terminal property's
        // value and the stored element is a commitment `Item` (an
        // `ItemWithSumItem` under a summable index) — mirror of the
        // insert-side branch in
        // `add_reference_for_index_level_for_contract_operations_v0`.
        // `terminal` can only be `Some` on a PV14+ indexOnly contract, so
        // this branch is unreachable for every historical document.
        if let Some(terminal_property) = index_type.terminal.as_deref() {
            key_info_path.push(KnownKey(vec![0]));

            let member_tree_type = terminal_member_tree_type(index_type);

            // Sum-bearing entries (`ItemWithSumItem`) carry the i64 sum
            // item alongside the commitment payload; mirror the insert
            // side's 10-byte worst case. Delete-side sum-decrement is
            // implicit: grovedb reads the amount off the stored element
            // and propagates the subtraction — same as stored-type
            // `ReferenceWithSumItem` entries below.
            let estimated_value_size = if index_type.summable.is_some() {
                crate::drive::document::INDEX_ONLY_ROW_COMMITMENT_SIZE + 10
            } else {
                crate::drive::document::INDEX_ONLY_ROW_COMMITMENT_SIZE
            };

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                estimated_costs_only_with_layer_info.insert(
                    key_info_path.clone(),
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

            // The member key: the terminal property's value off the
            // (synthesized) document; estimation-only calls fall back to
            // the event id, same as the docId fallback below.
            let member_key: Vec<u8> = match document_and_contract_info
                .owned_document_info
                .document_info
                .get_borrowed_document_and_storage_flags()
            {
                Some((document, _)) => document
                    .get_raw_for_document_type(
                        terminal_property,
                        document_type,
                        document_and_contract_info.owned_document_info.owner_id,
                        platform_version,
                    )?
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                        "indexOnly terminal value must be present on delete: the \
                         delete transition carries every property and the owner",
                    )))?,
                None => event_id.to_vec(),
            };

            let delete_apply_type = Self::stateless_delete_of_non_tree_for_costs(
                AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    estimated_value_size,
                    storage_flags.map(|s| s.serialized_size()),
                ),
                &key_info_path,
                // we know we are not deleting a tree
                Some(MaybeTree::NotTree),
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;

            // Drained groups prune upward exactly as with references, so
            // ranked secondaries drop the group when its last member goes.
            self.batch_delete_up_tree_while_empty(
                key_info_path,
                member_key.as_slice(),
                Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                delete_apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                &platform_version.drive,
            )?;

            return Ok(());
        }

        // unique indexes will be stored under key "0"
        // non unique indices should have a tree at key "0" that has all elements based off of primary key
        if index_type.index_type == NonUniqueIndex
            || index_type.index_type == ContestedResourceIndex
            || any_fields_null
        {
            key_info_path.push(KnownKey(vec![0]));

            // Mirror the insert path: tree variant is driven by the
            // composition of the index's countability AND summability —
            // `terminal_member_tree_type` is the shared dispatch (eight
            // cases over the v3 sum-tree-expanded TreeType set), used
            // by both sides so the delete's estimation layers cannot
            // drift from the trees the insert actually created.
            let reference_tree_type = terminal_member_tree_type(index_type);

            // Delete-side sum-decrement is implicit: under a summable
            // index, the existing reference at `[..., 0, doc_id]` is an
            // `Element::ItemWithSumItem(doc_id, amount_i64, flags)`
            // (written by the insert path). The contribution is
            // recovered from the reference element itself — Drive
            // never re-reads the source document at delete time
            // (the field's value may have drifted or the document
            // may not be deserializable anymore). grovedb's normal
            // delete propagation subtracts that `i64` from every
            // ancestor sum tree, so no sum-specific delete logic is
            // needed here.

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                // On this level we will have a 0 and all the top index paths
                estimated_costs_only_with_layer_info.insert(
                    key_info_path.clone(),
                    EstimatedLayerInformation {
                        tree_type: reference_tree_type,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            DEFAULT_HASH_SIZE_U8,
                            NoSumTrees,
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            let delete_apply_type = Self::stateless_delete_of_non_tree_for_costs(
                AllReference(
                    DEFAULT_HASH_SIZE_U8,
                    document_reference_size(document_type),
                    storage_flags.map(|s| s.serialized_size()),
                ),
                &key_info_path,
                // we know we are not deleting a tree
                Some(MaybeTree::NotTree),
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;

            // here we should return an error if the element already exists
            self.batch_delete_up_tree_while_empty(
                key_info_path,
                document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_document_id_as_slice()
                    .unwrap_or(event_id.as_slice()),
                Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                delete_apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                &platform_version.drive,
            )?;
        } else {
            let delete_apply_type = Self::stateless_delete_of_non_tree_for_costs(
                AllReference(
                    1,
                    document_reference_size(document_type),
                    storage_flags.map(|s| s.serialized_size()),
                ),
                &key_info_path,
                // we know we are not deleting a tree
                Some(MaybeTree::NotTree),
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
            // here we should return an error if the element already exists
            self.batch_delete_up_tree_while_empty(
                key_info_path,
                &[0],
                Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                delete_apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}
