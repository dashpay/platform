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
    /// Mints (issues) new tokens by increasing the total supply and adding them to an identity's balance.
    #[allow(clippy::too_many_arguments)]
    pub fn token_mint(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        issuance_amount: u64,
        allow_first_mint: bool,
        allow_saturation: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.token.update.mint {
            0 => self.token_mint_v0(
                token_id,
                identity_id,
                issuance_amount,
                allow_first_mint,
                allow_saturation,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_mint".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Adds the operations to mint tokens without calculating fees and optionally applying.
    pub fn token_mint_add_to_operations(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        issuance_amount: u64,
        allow_first_mint: bool,
        allow_saturation: bool,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version.drive.methods.token.update.mint {
            0 => self.token_mint_add_to_operations_v0(
                token_id,
                identity_id,
                issuance_amount,
                allow_first_mint,
                allow_saturation,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_mint_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Gathers the operations needed to mint tokens.
    #[allow(clippy::too_many_arguments)]
    pub fn token_mint_operations(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        issuance_amount: u64,
        allow_first_mint: bool,
        allow_saturation: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.update.mint {
            0 => self.token_mint_operations_v0(
                token_id,
                identity_id,
                issuance_amount,
                allow_first_mint,
                allow_saturation,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_mint_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
