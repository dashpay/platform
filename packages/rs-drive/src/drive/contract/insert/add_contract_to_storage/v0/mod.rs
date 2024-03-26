use crate::common::encode::encode_u64;
use crate::drive::contract::paths;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::{BatchInsertTreeApplyType, DirectQueryType};
use crate::drive::object_size_info::DriveKeyInfo::KeyRef;
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElementSize,
};
use crate::drive::object_size_info::PathKeyInfo;
use crate::drive::Drive;
use crate::drive::LowLevelDriveOperation;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::DataContract;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::SiblingReference;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds a contract to storage.
    #[inline(always)]
    pub(super) fn add_contract_to_storage_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        insert_operations: &mut Vec<LowLevelDriveOperation>,
        is_first_insert: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let contract_root_path = paths::contract_root_path(contract.id_ref().as_bytes());
        if contract.config().keeps_history() {
            let element_flags = contract_element.get_flags().clone();
            let storage_flags =
                StorageFlags::map_cow_some_element_flags_ref(contract_element.get_flags())?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
                    contract,
                    estimated_costs_only_with_layer_info,
                    drive_version,
                )?;
            }

            if is_first_insert {
                self.batch_insert_empty_tree(
                    contract_root_path,
                    KeyRef(&[0]),
                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                    insert_operations,
                    drive_version,
                )?;
            } else {
                let apply_type = if estimated_costs_only_with_layer_info.is_some() {
                    BatchInsertTreeApplyType::StatelessBatchInsertTree {
                        is_sum_tree: false,
                        in_tree_using_sums: false,
                        flags_len: storage_flags
                            .as_ref()
                            .map(|flags| flags.to_element_flags().len())
                            .unwrap_or_default() as u32,
                    }
                } else {
                    BatchInsertTreeApplyType::StatefulBatchInsertTree
                };

                let key_info = PathKeyInfo::PathFixedSizeKeyRef((contract_root_path, &[0]));

                self.batch_insert_empty_tree_if_not_exists(
                    key_info,
                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                    apply_type,
                    transaction,
                    &mut None,
                    insert_operations,
                    drive_version,
                )?;
            }

            let encoded_time = encode_u64(block_info.time_ms);
            let contract_keeping_history_storage_path =
                paths::contract_keeping_history_root_path(contract.id_ref().as_bytes());

            if !is_first_insert {
                // we can use a DirectQueryType::StatefulDirectQuery because if we were stateless we would always think
                // this was the first insert
                let maybe_element = self.grove_get_raw_optional(
                    (&contract_keeping_history_storage_path).into(),
                    encoded_time.as_slice(),
                    DirectQueryType::StatefulDirectQuery,
                    transaction,
                    insert_operations,
                    drive_version,
                )?;
                if maybe_element.is_some() {
                    return Err(Error::Drive(DriveError::UpdatingContractWithHistoryError(
                        "updating a contract with same time as a previous revision",
                    )));
                }
            };

            self.batch_insert(
                PathFixedSizeKeyRefElement((
                    contract_keeping_history_storage_path,
                    encoded_time.as_slice(),
                    contract_element,
                )),
                insert_operations,
                drive_version,
            )?;

            let reference_element =
                Element::Reference(SiblingReference(encoded_time), Some(1), element_flags);

            let path_key_element_info = if estimated_costs_only_with_layer_info.is_none() {
                PathFixedSizeKeyRefElement((
                    contract_keeping_history_storage_path,
                    &[0],
                    reference_element,
                ))
            } else {
                PathKeyElementSize((
                    KeyInfoPath::from_known_path(contract_keeping_history_storage_path),
                    KeyInfo::KnownKey(vec![0u8]),
                    reference_element,
                ))
            };
            self.batch_insert(path_key_element_info, insert_operations, drive_version)?;
        } else {
            // the contract is just stored at key 0
            let path_key_element_info = if estimated_costs_only_with_layer_info.is_none() {
                PathFixedSizeKeyRefElement((contract_root_path, &[0], contract_element))
            } else {
                PathKeyElementSize((
                    KeyInfoPath::from_known_path(contract_root_path),
                    KeyInfo::KnownKey(vec![0u8]),
                    contract_element,
                ))
            };
            self.batch_insert(path_key_element_info, insert_operations, drive_version)?;
        }
        Ok(())
    }
}
