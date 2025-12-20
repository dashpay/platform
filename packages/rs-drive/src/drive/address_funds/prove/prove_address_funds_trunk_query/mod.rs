//! Proves address funds using a trunk chunk query.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves address funds using a trunk chunk query.
    ///
    /// This function generates a trunk chunk proof for the address funds tree,
    /// useful for retrieving the initial structure of the tree up to the configured depth.
    ///
    /// # Parameters
    /// - `platform_version`: The version of the platform that determines the correct method version
    ///   and the trunk query depth.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The serialized proof bytes.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    pub fn prove_address_funds_trunk_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .prove_address_funds_trunk_query
        {
            0 => self.prove_address_funds_trunk_query_v0(platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_address_funds_trunk_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves address funds using a trunk chunk query and tracks operations.
    ///
    /// This function is similar to `prove_address_funds_trunk_query` but also adds
    /// operations to the drive for tracking costs.
    ///
    /// # Parameters
    /// - `drive_operations`: A mutable reference to a vector that stores low-level drive operations.
    /// - `platform_version`: The version of the platform that determines the correct method version
    ///   and the trunk query depth.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The serialized proof bytes.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    pub fn prove_address_funds_trunk_query_operations(
        &self,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .prove_address_funds_trunk_query
        {
            0 => self
                .prove_address_funds_trunk_query_operations_v0(drive_operations, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_address_funds_trunk_query_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
