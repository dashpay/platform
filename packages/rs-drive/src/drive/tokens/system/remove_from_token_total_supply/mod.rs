mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Removes from the token's total supply
    pub fn remove_from_token_total_supply(
        &self,
        token_id: [u8; 32],
        amount: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .token
            .update
            .remove_from_token_total_supply
        {
            0 => self.remove_from_token_total_supply_v0(
                token_id,
                amount,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_from_token_total_supply".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the operations of removing from the token total supply
    pub fn remove_from_token_total_supply_add_to_operations(
        &self,
        token_id: [u8; 32],
        amount: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .token
            .update
            .remove_from_token_total_supply
        {
            0 => self.remove_from_token_total_supply_add_to_operations_v0(
                token_id,
                amount,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_from_token_total_supply_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations needed to remove from the token total supply
    pub fn remove_from_token_total_supply_operations(
        &self,
        token_id: [u8; 32],
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .update
            .remove_from_token_total_supply
        {
            0 => self.remove_from_token_total_supply_operations_v0(
                token_id,
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_from_token_total_supply_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
