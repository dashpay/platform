use crate::drive::document::unique_event_id;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;

use crate::util::grove_operations::BatchInsertTreeApplyType;

use crate::drive::Drive;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo};

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

use dpp::version::PlatformVersion;

use crate::drive::document::estimation_costs::estimated_sum_trees_for_value_tree_type::estimated_sum_trees_for_value_tree_type;
use crate::drive::document::index_level_tree_types::index_level_tree_types_with_continuation_demotion;
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
    pub(super) fn add_indices_for_top_index_level_for_contract_operations_v2(
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
        let drive_version = &platform_version.drive;
        let index_level = &document_and_contract_info.document_type.index_structure();
        let contract = document_and_contract_info.contract;
        let event_id = unique_event_id();
        let document_type = document_and_contract_info.document_type;
        let storage_flags =
            if document_type.documents_mutable() || contract.config().can_be_deleted() {
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
            let tree_types = index_level_tree_types_with_continuation_demotion(sub_level);
            let property_name_tree_type = tree_types.property_name_tree_type;
            let value_tree_type = tree_types.value_tree_type;

            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path: Vec<Vec<u8>> = contract_document_type_path.clone();
            index_path.push(Vec::from(name.as_bytes()));

            // with the example of the dashpay contract's first index
            // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId
            let document_top_field = document_and_contract_info
                .owned_document_info
                .document_info
                .get_raw_for_document_type(
                    name,
                    document_type,
                    document_and_contract_info.owned_document_info.owner_id,
                    Some((sub_level, event_id)),
                    platform_version,
                )?
                .unwrap_or_default();

            // The zero will not matter here, because the PathKeyInfo is variable
            let path_key_info = document_top_field.clone().add_path::<0>(index_path.clone());
            // here we are inserting the value tree (per distinct property value)
            // under the top-level property-name tree. The top-level property-name
            // tree itself is created at contract setup, so the apply_type's
            // `in_tree_type` reflects whichever variant the contract setup used.
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
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info.clone(),
                value_tree_type,
                storage_flags,
                value_apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                drive_version,
            )?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type, platform_version)?;

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

            let mut index_path_info = if document_and_contract_info
                .owned_document_info
                .document_info
                .is_document_size()
            {
                // This is a stateless operation
                PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(index_path))
            } else {
                PathInfo::PathAsVec::<0>(index_path)
            };

            // we push the actual value of the index path
            index_path_info.push(document_top_field)?;
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
        Ok(())
    }
}
