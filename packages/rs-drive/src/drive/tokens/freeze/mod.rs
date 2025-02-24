mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Burns tokens by reducing the total supply and removing them from an identity's balance.
    pub fn token_freeze(
        &self,
        token_id: Identifier,
        frozen_identity_id: Identifier,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.token.update.freeze {
            0 => self.token_freeze_v0(
                token_id,
                frozen_identity_id,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_freeze".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the operations to freeze tokens without calculating fees and optionally applying.
    pub fn token_freeze_add_to_operations(
        &self,
        token_id: Identifier,
        frozen_identity_id: Identifier,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version.drive.methods.token.update.freeze {
            0 => self.token_freeze_add_to_operations_v0(
                token_id,
                frozen_identity_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_freeze_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Gathers the operations needed to freeze tokens.
    pub fn token_freeze_operations(
        &self,
        token_id: Identifier,
        frozen_identity_id: Identifier,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.update.freeze {
            0 => self.token_freeze_operations_v0(
                token_id,
                frozen_identity_id,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_freeze_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
