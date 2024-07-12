mod v0;

use crate::drive::flags::StorageFlags;

use crate::drive::object_size_info::{DocumentAndContractInfo, PathInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;

use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds the terminal reference.
    pub fn add_contested_reference_and_vote_subtree_to_document_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        storage_flags: Option<&StorageFlags>,
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
            .insert_contested
            .add_contested_reference_and_vote_subtree_to_document_operations
        {
            0 => self.add_contested_reference_and_vote_subtree_to_document_operations_v0(
                document_and_contract_info,
                index_path_info,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contested_reference_for_index_level_for_contract_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
