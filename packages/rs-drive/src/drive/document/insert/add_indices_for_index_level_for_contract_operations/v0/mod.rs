use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentPropertyType;
use dpp::data_contract::document_type::IndexLevel;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds indices for an index level and recurses.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_indices_for_index_level_for_contract_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_level: &IndexLevel,
        mut any_fields_null: bool,
        mut all_fields_null: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        event_id: [u8; 32],
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(index_type) = index_level.has_index_with_type() {
            self.add_reference_for_index_level_for_contract_operations(
                document_and_contract_info,
                index_path_info.clone(),
                index_type,
                any_fields_null,
                all_fields_null,
                previous_batch_operations,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        let document_type = document_and_contract_info.document_type;

        let sub_level_index_count = index_level.sub_levels().len() as u32;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
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

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: storage_flags
                    .map(|s| s.serialized_size())
                    .unwrap_or_default(),
            }
        };

        // fourth we need to store a reference to the document for each index
        for (name, sub_level) in index_level.sub_levels() {
            let mut sub_level_index_path_info = index_path_info.clone();
            let index_property_key = KeyRef(name.as_bytes());

            // Check if this property is an array type
            let is_array_property = document_type
                .flattened_properties()
                .get(name)
                .map(|prop| {
                    matches!(
                        prop.property_type,
                        DocumentPropertyType::Array(_) | DocumentPropertyType::VariableTypeArray(_)
                    )
                })
                .unwrap_or(false);

            // Insert the property name tree (e.g., "hashtags")
            let path_key_info = index_property_key
                .clone()
                .add_path_info(sub_level_index_path_info.clone());

            self.batch_insert_empty_tree_if_not_exists(
                path_key_info.clone(),
                TreeType::NormalTree,
                *storage_flags,
                apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                &platform_version.drive,
            )?;

            sub_level_index_path_info.push(index_property_key)?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type, platform_version)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document top field is too big for being an index",
                    )));
                }

                estimated_costs_only_with_layer_info.insert(
                    sub_level_index_path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        tree_type: TreeType::NormalTree,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            document_top_field_estimated_size as u8,
                            NoSumTrees,
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            if is_array_property {
                // Handle array property - create index entries for each element
                let array_elements = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_raw_array_elements_for_document_type(
                        name,
                        document_type,
                        platform_version,
                    )?;

                if array_elements.is_empty() {
                    // Empty array has zero elements to index, so we don't create any entries.
                    // This differs from null scalars which create an entry with an empty key.
                    // No recursion needed since there are no elements to pair with sub-level fields.
                    continue;
                }

                // For each array element, create an index entry
                for element_value in array_elements {
                    let element_key = Key(element_value);
                    let mut element_path_info = sub_level_index_path_info.clone();

                    // Insert tree for this element value
                    let element_path_key_info =
                        element_key.clone().add_path_info(element_path_info.clone());

                    self.batch_insert_empty_tree_if_not_exists(
                        element_path_key_info,
                        TreeType::NormalTree,
                        *storage_flags,
                        apply_type,
                        transaction,
                        previous_batch_operations,
                        batch_operations,
                        &platform_version.drive,
                    )?;

                    // Push element value to path and recurse
                    element_path_info.push(element_key)?;

                    self.add_indices_for_index_level_for_contract_operations_v0(
                        document_and_contract_info,
                        element_path_info,
                        sub_level,
                        any_fields_null,
                        false, // Not all fields null since we have an element
                        previous_batch_operations,
                        storage_flags,
                        estimated_costs_only_with_layer_info,
                        event_id,
                        transaction,
                        batch_operations,
                        platform_version,
                    )?;
                }
            } else {
                // Handle scalar property - existing logic
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

                let path_key_info = document_index_field
                    .clone()
                    .add_path_info(sub_level_index_path_info.clone());

                // Insert tree for this field value
                self.batch_insert_empty_tree_if_not_exists(
                    path_key_info.clone(),
                    TreeType::NormalTree,
                    *storage_flags,
                    apply_type,
                    transaction,
                    previous_batch_operations,
                    batch_operations,
                    &platform_version.drive,
                )?;

                any_fields_null |= document_index_field.is_empty();
                all_fields_null &= document_index_field.is_empty();

                // Push the actual value of the index path
                sub_level_index_path_info.push(document_index_field)?;

                self.add_indices_for_index_level_for_contract_operations_v0(
                    document_and_contract_info,
                    sub_level_index_path_info,
                    sub_level,
                    any_fields_null,
                    all_fields_null,
                    previous_batch_operations,
                    storage_flags,
                    estimated_costs_only_with_layer_info,
                    event_id,
                    transaction,
                    batch_operations,
                    platform_version,
                )?;
            }
        }
        Ok(())
    }
}
