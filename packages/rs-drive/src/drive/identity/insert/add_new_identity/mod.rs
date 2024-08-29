use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Adds a identity by inserting a new identity subtree structure to the `Identities` subtree.
    pub fn add_new_identity(
        &self,
        identity: Identity,
        is_masternode_identity: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .insert
            .add_new_identity
        {
            0 => self.add_new_identity_v0(
                identity,
                is_masternode_identity,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds identity creation operations to drive operations
    pub fn add_new_identity_add_to_operations(
        &self,
        identity: Identity,
        is_masternode_identity: bool,
        block_info: &BlockInfo,
        apply: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .insert
            .add_new_identity
        {
            0 => self.add_new_identity_add_to_operations_v0(
                identity,
                is_masternode_identity,
                block_info,
                apply,
                previous_batch_operations,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_identity_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations needed to create an identity
    pub fn add_new_identity_operations(
        &self,
        identity: Identity,
        is_masternode_identity: bool,
        block_info: &BlockInfo,
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
            .identity
            .insert
            .add_new_identity
        {
            0 => self.add_new_identity_operations_v0(
                identity,
                is_masternode_identity,
                block_info,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_identity_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
