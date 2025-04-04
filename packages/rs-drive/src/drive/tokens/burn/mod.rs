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
    /// Burns tokens by reducing the total supply and removing them from an identity's balance.
    #[allow(clippy::too_many_arguments)]
    pub fn token_burn(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        burn_amount: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.token.update.burn {
            0 => self.token_burn_v0(
                token_id,
                identity_id,
                burn_amount,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_burn".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Adds the operations to burn tokens without calculating fees and optionally applying.
    pub fn token_burn_add_to_operations(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        burn_amount: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version.drive.methods.token.update.burn {
            0 => self.token_burn_add_to_operations_v0(
                token_id,
                identity_id,
                burn_amount,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_burn_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Gathers the operations needed to burn tokens.
    pub fn token_burn_operations(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        burn_amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.update.burn {
            0 => self.token_burn_operations_v0(
                token_id,
                identity_id,
                burn_amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_burn_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
