use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};

use dpp::data_contract::document_type::IndexLevel;

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use crate::drive::document::estimation_costs::estimated_sum_trees_for_value_tree_type::estimated_sum_trees_for_value_tree_type;
use crate::drive::document::index_level_tree_types::index_level_tree_types_with_continuation_demotion;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;

use crate::util::storage_flags::StorageFlags;

use crate::util::object_size_info::DriveKeyInfo::KeyRef;

use crate::drive::Drive;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo};

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;

impl Drive {
    /// Removes indices for an index level and recurses.
    ///
    /// v2 derives tree types through the shared
    /// [`index_level_tree_types_with_continuation_demotion`] helper so
    /// the estimation layer info describes the exact on-disk shape the
    /// v2 insert walker writes — including the continuation demotion of
    /// provable count-bearing value trees to `CountSumTree`. Must stay
    /// in lockstep with
    /// [`Drive::add_indices_for_index_level_for_contract_operations_v2`];
    /// part of the platform v14 shared-prefix aggregate fix.
    ///
    /// The delete path constructs no wrapper elements itself — grovedb
    /// looks through `NonCounted` / `NotSummed` / `NotCountedOrSummed`
    /// when deleting trees and subtracts the stored (zero) feature
    /// contribution, so only the tree-type derivation needs to mirror
    /// the insert side.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_indices_for_index_level_for_contract_operations_v2(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_level: &IndexLevel,
        mut any_fields_null: bool,
        mut all_fields_null: bool,
        parent_value_tree_type: TreeType,
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
        let sub_level_index_count = index_level.sub_levels().len() as u32;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have a 0 and all the top index paths.
            // `parent_value_tree_type` carries the (post-demotion)
            // TreeType the v2 insert walker actually wrote.
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
                EstimatedLayerInformation {
                    tree_type: parent_value_tree_type,
                    estimated_layer_count: ApproximateElements(sub_level_index_count + 1),
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        if let Some(index_type) = index_level.has_index_with_type() {
            self.remove_reference_for_index_level_for_contract_operations(
                document_and_contract_info,
                index_path_info.clone(),
                index_type,
                any_fields_null,
                all_fields_null,
                storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            )?;
        }

        let document_type = document_and_contract_info.document_type;

        // fourth we need to store a reference to the document for each index
        for (name, sub_level) in index_level.sub_levels() {
            // The delete walker writes nothing itself, but its
            // estimation layers must describe the tree the insert path
            // actually laid down — including the meta-schema-v3 ranked
            // upgrade of the property-name tree — or dry-run delete fees
            // drift from applied ones on ranked indexes.
            let tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
            let property_name_tree_type = tree_types.property_name_tree_type;
            let value_tree_type = tree_types.value_tree_type;

            let mut sub_level_index_path_info = index_path_info.clone();
            let index_property_key = KeyRef(name.as_bytes());

            let document_index_field = document_and_contract_info
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

            sub_level_index_path_info.push(index_property_key)?;

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

                // The property-name layer's children are value trees of
                // type `value_tree_type` (post-demotion — matching what
                // the v2 insert walker actually writes).
                estimated_costs_only_with_layer_info.insert(
                    sub_level_index_path_info.clone().convert_to_key_info_path(),
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

            // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
            // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

            any_fields_null |= document_index_field.is_empty();
            all_fields_null &= document_index_field.is_empty();

            // we push the actual value of the index path
            sub_level_index_path_info.push(document_index_field)?;
            // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
            // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            self.remove_indices_for_index_level_for_contract_operations_v2(
                document_and_contract_info,
                sub_level_index_path_info,
                sub_level,
                any_fields_null,
                all_fields_null,
                value_tree_type,
                storage_flags,
                previous_batch_operations,
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
