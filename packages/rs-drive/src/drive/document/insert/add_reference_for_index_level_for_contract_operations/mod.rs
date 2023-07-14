mod v0;

use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, STORAGE_FLAGS_SIZE};
use crate::drive::document::{document_reference_size, make_document_reference};
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::drive::object_size_info::DocumentInfo::{
    DocumentAndSerialization, DocumentEstimatedAverageSize, DocumentOwnedInfo,
    DocumentRefAndSerialization, DocumentRefInfo,
};
use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::drive::object_size_info::KeyElementInfo::{KeyElement, KeyUnknownElementSize};
use crate::drive::object_size_info::{DocumentAndContractInfo, PathInfo, PathKeyElementInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds the terminal reference.
    pub fn add_reference_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        mut index_path_info: PathInfo<0>,
        unique: bool,
        any_fields_null: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .document
            .insert
            .add_reference_for_index_level_for_contract_operations
        {
            0 => self.add_reference_for_index_level_for_contract_operations_v0(
                document_and_contract_info,
                index_path_info,
                unique,
                any_fields_null,
                previous_batch_operations,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_reference_for_index_level_for_contract_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
