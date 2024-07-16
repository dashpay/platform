mod v0;

use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::{DocumentAndContractInfo, PathInfo};
use dpp::version::drive_versions::DriveVersion;

use grovedb::batch::KeyInfoPath;

use dpp::data_contract::document_type::IndexType;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds the terminal reference.
    pub fn add_reference_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_type: IndexType,
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
                index_type,
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
