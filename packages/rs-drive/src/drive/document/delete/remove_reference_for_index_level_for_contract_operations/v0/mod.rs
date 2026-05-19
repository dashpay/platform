use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::{AllReference, AllSubtrees};
use grovedb::{EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};

use dpp::data_contract::document_type::IndexType::{ContestedResourceIndex, NonUniqueIndex};
use dpp::data_contract::document_type::{IndexCountability, IndexLevelTypeInfo};
use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use crate::drive::constants::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::document::document_reference_size;
use crate::util::storage_flags::StorageFlags;

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

        // unique indexes will be stored under key "0"
        // non unique indices should have a tree at key "0" that has all elements based off of primary key
        if index_type.index_type == NonUniqueIndex
            || index_type.index_type == ContestedResourceIndex
            || any_fields_null
        {
            key_info_path.push(KnownKey(vec![0]));

            // Mirror the insert path: tree variant is driven by the
            // composition of the index's countability AND summability.
            // See the matching dispatch table in
            // `add_reference_for_index_level_for_contract_operations_v0`
            // — eight cases over the v3 sum-tree-expanded TreeType
            // set (NormalTree / CountTree / ProvableCountTree /
            // SumTree / ProvableSumTree / CountSumTree /
            // ProvableCountSumTree / ProvableCountProvableSumTree).
            let count_provable = matches!(
                index_type.countable,
                IndexCountability::CountableAllowingOffset
            );
            let count_root_only =
                matches!(index_type.countable, IndexCountability::Countable) && !count_provable;
            let sum_provable = index_type.range_summable;
            let sum_root_only = index_type.summable.is_some() && !sum_provable;
            let want_count = count_provable || count_root_only;
            let want_sum = sum_provable || sum_root_only;
            let reference_tree_type =
                match (count_provable, count_root_only, sum_provable, sum_root_only) {
                    (false, false, false, false) => TreeType::NormalTree,
                    (false, true, false, false) => TreeType::CountTree,
                    (true, _, false, false) => TreeType::ProvableCountTree,
                    (false, false, false, true) => TreeType::SumTree,
                    (false, false, true, _) => TreeType::ProvableSumTree,
                    (false, true, false, true) => TreeType::CountSumTree,
                    (true, _, false, true) => TreeType::ProvableCountSumTree,
                    (true, _, true, _) => TreeType::ProvableCountProvableSumTree,
                    (false, true, true, _) => TreeType::ProvableCountProvableSumTree,
                };
            let _ = (want_count, want_sum); // narrative parity with the dispatch table.

            // Delete-side sum-decrement is implicit: when `want_sum`,
            // the existing reference at `[..., 0, doc_id]` is an
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
