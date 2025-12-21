//! Module for verifying address funds trunk chunk queries.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::GroveTrunkQueryResult;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies a trunk chunk proof for the address funds tree.
    ///
    /// This method validates a trunk chunk proof and returns the verified result
    /// containing the elements and leaf keys for subsequent branch queries.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the serialized trunk chunk proof.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, GroveTrunkQueryResult))`: On success, returns a tuple containing:
    ///   - `RootHash`: The root hash confirming the proof's validity.
    ///   - `GroveTrunkQueryResult`: The verified trunk query result containing elements and leaf keys.
    /// - `Err(Error)`: If verification fails.
    ///
    /// # Errors
    /// - [`Error::Proof`]: If the proof is invalid or corrupted.
    /// - [`Error::Drive(DriveError::UnknownVersionMismatch)`]: If called with an unsupported platform version.
    pub fn verify_address_funds_trunk_query(
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, GroveTrunkQueryResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .address_funds
            .verify_address_funds_trunk_query
        {
            0 => Self::verify_address_funds_trunk_query_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_address_funds_trunk_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
