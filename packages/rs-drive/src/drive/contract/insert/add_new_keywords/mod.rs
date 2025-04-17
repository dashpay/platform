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
    /// Creates the documents in the Keyword Search contract for the contract keywords and
    /// returns the fee result
    pub fn add_new_contract_keywords(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.contract.insert.add_keywords {
            0 => self.add_new_contract_keywords_v0(
                contract_id,
                owner_id,
                keywords,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_keywords".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Creates and applies the LowLeveLDriveOperations needed to create
    /// the documents in the Keyword Search contract for the contract keywords
    pub fn add_new_contract_keywords_add_to_operations(
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
        match platform_version.drive.methods.contract.insert.add_keywords {
            0 => self.add_new_contract_keywords_add_to_operations_v0(
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
                method: "add_new_keywords_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Creates and returns the LowLeveLDriveOperations needed to create
    /// the documents in the Keyword Search contract for the contract keywords
    pub fn add_new_contract_keywords_operations(
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
        match platform_version.drive.methods.contract.insert.add_keywords {
            0 => self.add_new_contract_keywords_operations_v0(
                contract_id,
                owner_id,
                keywords,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_keywords_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
