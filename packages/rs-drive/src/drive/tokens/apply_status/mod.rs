mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Sets a token status
    pub fn token_apply_status(
        &self,
        token_id: [u8; 32],
        status: TokenStatus,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.token.update.apply_status {
            0 => self.token_apply_status_v0(
                token_id,
                status,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_apply_status".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the operations to apply_status tokens without calculating fees and optionally applying.
    pub fn token_apply_status_add_to_operations(
        &self,
        token_id: [u8; 32],
        status: TokenStatus,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version.drive.methods.token.update.apply_status {
            0 => self.token_apply_status_add_to_operations_v0(
                token_id,
                status,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_apply_status_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Gathers the operations needed to apply_status tokens.
    pub fn token_apply_status_operations(
        &self,
        token_id: [u8; 32],
        status: TokenStatus,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.update.apply_status {
            0 => self.token_apply_status_operations_v0(
                token_id,
                status,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_apply_status_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
