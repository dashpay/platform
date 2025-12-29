//! Proves address funds using a branch chunk query.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::BlockHeight;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves address funds using a branch chunk query.
    ///
    /// This function generates a branch chunk proof for navigating deeper into
    /// the address funds tree after an initial trunk query.
    ///
    /// # Parameters
    /// - `key`: The key to navigate to in the address funds tree before extracting the branch.
    /// - `depth`: The depth of the branch to return from the key. Must be between
    ///   `address_funds_query_min_depth` and `address_funds_query_max_depth` (inclusive).
    /// - `checkpoint_height`: Block height of the checkpoint to use.
    ///   This should match the height from the trunk query response to ensure consistency.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The serialized proof bytes.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    /// - `DriveError::InvalidInput`: If the depth is outside the allowed range.
    /// - `DriveError::CheckpointNotFound`: If the specified checkpoint height doesn't exist.
    pub fn prove_address_funds_branch_query(
        &self,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .prove_address_funds_branch_query
        {
            0 => self.prove_address_funds_branch_query_v0(
                key,
                depth,
                checkpoint_height,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_address_funds_branch_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves address funds using a branch chunk query and tracks operations.
    ///
    /// This function is similar to `prove_address_funds_branch_query` but also adds
    /// operations to the drive for tracking costs.
    ///
    /// # Parameters
    /// - `key`: The key to navigate to in the address funds tree before extracting the branch.
    /// - `depth`: The depth of the branch to return from the key. Must be between
    ///   `address_funds_query_min_depth` and `address_funds_query_max_depth` (inclusive).
    /// - `checkpoint_height`: Block height of the checkpoint to use.
    ///   This should match the height from the trunk query response to ensure consistency.
    /// - `drive_operations`: A mutable reference to a vector that stores low-level drive operations.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The serialized proof bytes.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    /// - `DriveError::InvalidInput`: If the depth is outside the allowed range.
    /// - `DriveError::CheckpointNotFound`: If the specified checkpoint height doesn't exist.
    pub fn prove_address_funds_branch_query_operations(
        &self,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .prove_address_funds_branch_query
        {
            0 => self.prove_address_funds_branch_query_operations_v0(
                key,
                depth,
                checkpoint_height,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_address_funds_branch_query_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
