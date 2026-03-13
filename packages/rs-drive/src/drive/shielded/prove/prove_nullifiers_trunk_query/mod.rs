//! Proves nullifiers using a trunk chunk query.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves nullifiers using a trunk chunk query.
    ///
    /// This function generates a trunk chunk proof for the nullifiers tree of a shielded pool,
    /// useful for retrieving the initial structure of the tree up to the configured depth.
    ///
    /// # Parameters
    /// - `pool_type`: The shielded pool type (0 = credit, 1 = main token, 2 = individual token).
    /// - `pool_identifier`: Optional 32-byte identifier for individual token pools (pool_type=2).
    /// - `platform_version`: The version of the platform that determines the correct method version
    ///   and the trunk query depth.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The serialized proof bytes.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    pub fn prove_nullifiers_trunk_query(
        &self,
        pool_type: u32,
        pool_identifier: Option<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .shielded
            .prove_nullifiers_trunk_query
        {
            0 => self.prove_nullifiers_trunk_query_v0(pool_type, pool_identifier, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_nullifiers_trunk_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves nullifiers using a trunk chunk query and tracks operations.
    pub fn prove_nullifiers_trunk_query_operations(
        &self,
        pool_type: u32,
        pool_identifier: Option<Vec<u8>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .shielded
            .prove_nullifiers_trunk_query
        {
            0 => self.prove_nullifiers_trunk_query_operations_v0(
                pool_type,
                pool_identifier,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_nullifiers_trunk_query_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
