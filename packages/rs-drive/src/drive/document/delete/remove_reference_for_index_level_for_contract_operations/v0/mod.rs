use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::{AllReference, AllSubtrees};
use grovedb::{EstimatedLayerInformation, TransactionArg};

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use crate::drive::defaults::{CONTRACT_DOCUMENTS_PATH_HEIGHT, DEFAULT_HASH_SIZE_U8};
use crate::drive::document::document_reference_size;
use crate::drive::flags::StorageFlags;

use crate::drive::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo};
use crate::drive::Drive;

use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;

impl Drive {
    /// Removes the terminal reference.
    #[inline(always)]
    pub(super) fn remove_reference_for_index_level_for_contract_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        unique: bool,
        any_fields_null: bool,
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
        let mut key_info_path = index_path_info.convert_to_key_info_path();

        let document_type = document_and_contract_info.document_type;

        // unique indexes will be stored under key "0"
        // non unique indices should have a tree at key "0" that has all elements based off of primary key
        if !unique || any_fields_null {
            key_info_path.push(KnownKey(vec![0]));

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                // On this level we will have a 0 and all the top index paths
                estimated_costs_only_with_layer_info.insert(
                    key_info_path.clone(),
                    EstimatedLayerInformation {
                        is_sum_tree: false,
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
                Some((false, false)),
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
                Some((false, false)),
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
