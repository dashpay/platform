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
    /// Transfers tokens from one identity to another without changing total supply.
    #[allow(clippy::too_many_arguments)]
    pub fn token_transfer(
        &self,
        token_id: [u8; 32],
        from_identity_id: [u8; 32],
        to_identity_id: [u8; 32],
        amount: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.token.update.transfer {
            0 => self.token_transfer_v0(
                token_id,
                from_identity_id,
                to_identity_id,
                amount,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_transfer".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Adds operations to transfer tokens without calculating fees.
    pub fn token_transfer_add_to_operations(
        &self,
        token_id: [u8; 32],
        from_identity_id: [u8; 32],
        to_identity_id: [u8; 32],
        amount: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version.drive.methods.token.update.transfer {
            0 => self.token_transfer_add_to_operations_v0(
                token_id,
                from_identity_id,
                to_identity_id,
                amount,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_transfer_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Gathers the operations needed to transfer tokens.
    #[allow(clippy::too_many_arguments)]
    pub fn token_transfer_operations(
        &self,
        token_id: [u8; 32],
        from_identity_id: [u8; 32],
        to_identity_id: [u8; 32],
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.update.transfer {
            0 => self.token_transfer_operations_v0(
                token_id,
                from_identity_id,
                to_identity_id,
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_transfer_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
