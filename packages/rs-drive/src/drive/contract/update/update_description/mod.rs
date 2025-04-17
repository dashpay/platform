use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

use dpp::prelude::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Updates the documents in the Keyword Search contract for the contract
    /// update description and returns the fee result
    #[allow(clippy::too_many_arguments)]
    pub fn update_contract_description(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        description: &String,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .update
            .update_description
        {
            0 => self.update_contract_description_v0(
                contract_id,
                owner_id,
                description,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_contract_description".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Creates and applies the LowLeveLDriveOperations needed to update
    /// the documents in the Keyword Search contract for the contract description
    #[allow(clippy::too_many_arguments)]
    pub fn update_contract_description_add_to_operations(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        description: &String,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .update
            .update_description
        {
            0 => self.update_contract_description_add_to_operations_v0(
                contract_id,
                owner_id,
                description,
                block_info,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_contract_description_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Creates and returns the LowLeveLDriveOperations needed to update
    /// the documents in the Keyword Search contract for the contract description
    #[allow(clippy::too_many_arguments)]
    pub fn update_contract_description_operations(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        description: &String,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .update
            .update_description
        {
            0 => self.update_contract_description_operations_v0(
                contract_id,
                owner_id,
                description,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_contract_description_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
