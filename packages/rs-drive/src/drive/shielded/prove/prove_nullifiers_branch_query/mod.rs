//! Proves nullifiers using a branch chunk query.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::BlockHeight;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves nullifiers using a branch chunk query.
    ///
    /// This function generates a branch chunk proof for navigating deeper into
    /// the nullifiers tree after an initial trunk query.
    ///
    /// # Parameters
    /// - `pool_type`: The shielded pool type (0 = credit, 1 = main token, 2 = individual token).
    /// - `pool_identifier`: Optional 32-byte identifier for individual token pools (pool_type=2).
    /// - `key`: The key to navigate to in the nullifiers tree before extracting the branch.
    /// - `depth`: The depth of the branch to return from the key.
    /// - `checkpoint_height`: Block height of the checkpoint to use for consistency.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The serialized proof bytes.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    pub fn prove_nullifiers_branch_query(
        &self,
        pool_type: u32,
        pool_identifier: Option<Vec<u8>>,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .shielded
            .prove_nullifiers_branch_query
        {
            0 => self.prove_nullifiers_branch_query_v0(
                pool_type,
                pool_identifier,
                key,
                depth,
                checkpoint_height,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_nullifiers_branch_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves nullifiers using a branch chunk query and tracks operations.
    pub fn prove_nullifiers_branch_query_operations(
        &self,
        pool_type: u32,
        pool_identifier: Option<Vec<u8>>,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .shielded
            .prove_nullifiers_branch_query
        {
            0 => self.prove_nullifiers_branch_query_operations_v0(
                pool_type,
                pool_identifier,
                key,
                depth,
                checkpoint_height,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_nullifiers_branch_query_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
