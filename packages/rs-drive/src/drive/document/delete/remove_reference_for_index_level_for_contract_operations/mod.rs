mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::{DocumentAndContractInfo, PathInfo};
use crate::util::storage_flags::StorageFlags;

use dpp::version::PlatformVersion;

use grovedb::batch::KeyInfoPath;

use dpp::data_contract::document_type::IndexLevelTypeInfo;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Removes the terminal reference.
    ///
    /// # Parameters
    /// * `document_and_contract_info`: The document and contract info.
    /// * `index_path_info`: Path info for the index.
    /// * `unique`: Whether the reference is unique.
    /// * `any_fields_null`: Whether any fields are null.
    /// * `storage_flags`: Optional storage flags.
    /// * `previous_batch_operations`: Previous batch operations to include.
    /// * `estimated_costs_only_with_layer_info`: Estimated costs with layer info.
    /// * `event_id`: The event ID.
    /// * `transaction`: The transaction argument.
    /// * `batch_operations`: The batch operations to include.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub(super) fn remove_reference_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_type: IndexLevelTypeInfo,
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
        match platform_version
            .drive
            .methods
            .document
            .delete
            .remove_reference_for_index_level_for_contract_operations
        {
            0 => self.remove_reference_for_index_level_for_contract_operations_v0(
                document_and_contract_info,
                index_path_info,
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
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_reference_for_index_level_for_contract_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
