mod v0;

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
use dpp::data_contract::DataContract;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::SiblingReference;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    pub(in crate::drive::contract) fn add_contract_to_storage(
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
        match drive_version
            .methods
            .contract
            .insert
            .add_contract_to_storage
        {
            0 => self.add_contract_to_storage_v0(
                contract_element,
                contract,
                block_info,
                estimated_costs_only_with_layer_info,
                insert_operations,
                is_first_insert,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_to_storage".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
