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

mod v1;

impl Drive {
    /// Updates keywords in the state.
    pub fn update_contract_keywords(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .insert
            .insert_contract
        {
            0 => Err(Error::Drive(DriveError::NotSupported(
                "Contract keywords are not supported in this version",
            ))),
            1 => self.update_contract_keywords_v0(
                contract_id,
                owner_id,
                keywords,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_keywords".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Adds keywords creation operations to drive operations
    pub fn update_contract_keywords_add_to_operations(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
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
            .insert
            .insert_contract
        {
            0 => Err(Error::Drive(DriveError::NotSupported(
                "Contract keywords are not supported in this version",
            ))),
            1 => self.update_contract_keywords_add_to_operations_v0(
                contract_id,
                owner_id,
                keywords,
                block_info,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_keywords_add_to_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// The operations needed to update keywords
    pub fn update_contract_keywords_operations(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
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
            .update_contract
        {
            0 => Err(Error::Drive(DriveError::NotSupported(
                "Contract keywords are not supported in this version",
            ))),
            1 => self.update_contract_keywords_operations_v0(
                contract_id,
                owner_id,
                keywords,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_keywords_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
