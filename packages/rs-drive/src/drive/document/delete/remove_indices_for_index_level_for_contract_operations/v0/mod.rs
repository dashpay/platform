use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::data_contract::document_type::IndexLevel;

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use crate::drive::defaults::DEFAULT_HASH_SIZE_U8;

use crate::drive::flags::StorageFlags;

use crate::drive::object_size_info::DriveKeyInfo::KeyRef;

use crate::drive::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo};
use crate::drive::Drive;

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;

impl Drive {
    /// Removes indices for an index level and recurses.
    #[inline]
    pub(super) fn remove_indices_for_index_level_for_contract_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_level: &IndexLevel,
        mut any_fields_null: bool,
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
            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(sub_level_index_count + 1),
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        if let Some(unique) = index_level.has_index_with_uniqueness() {
            self.remove_reference_for_index_level_for_contract_operations(
                document_and_contract_info,
                index_path_info.clone(),
                unique,
                any_fields_null,
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
                    .get_estimated_size_for_document_type(name, document_type)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document field is too big for being an index",
                    )));
                }

                estimated_costs_only_with_layer_info.insert(
                    sub_level_index_path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        is_sum_tree: false,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            document_top_field_estimated_size as u8,
                            NoSumTrees,
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
            // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

            any_fields_null |= document_index_field.is_empty();

            // we push the actual value of the index path
            sub_level_index_path_info.push(document_index_field)?;
            // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
            // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            self.remove_indices_for_index_level_for_contract_operations_v0(
                document_and_contract_info,
                sub_level_index_path_info,
                sub_level,
                any_fields_null,
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
