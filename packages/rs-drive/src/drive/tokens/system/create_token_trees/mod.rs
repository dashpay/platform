mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds an identity by inserting a new identity subtree structure to the `Identities` subtree.
    pub fn create_token_trees(
        &self,
        token_id: [u8; 32],
        start_as_paused: bool,
        allow_already_exists: bool,
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
            .create_token_trees
        {
            0 => self.create_token_trees_v0(
                token_id,
                start_as_paused,
                allow_already_exists,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "create_token_trees".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds identity creation operations to drive operations
    pub fn create_token_trees_add_to_operations(
        &self,
        token_id: [u8; 32],
        start_as_paused: bool,
        allow_already_exists: bool,
        apply: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .token
            .update
            .create_token_trees
        {
            0 => self.create_token_trees_add_to_operations_v0(
                token_id,
                start_as_paused,
                allow_already_exists,
                apply,
                previous_batch_operations,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "create_token_trees_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations needed to create an identity
    pub fn create_token_trees_operations(
        &self,
        token_id: [u8; 32],
        start_as_paused: bool,
        allow_already_exists: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
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
            .create_token_trees
        {
            0 => self.create_token_trees_operations_v0(
                token_id,
                start_as_paused,
                allow_already_exists,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "create_token_trees_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
