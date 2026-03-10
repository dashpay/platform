//! Module for verifying nullifiers trunk chunk queries.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::GroveTrunkQueryResult;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies a trunk chunk proof for the nullifiers tree of a shielded pool.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the serialized trunk chunk proof.
    /// - `pool_type`: The shielded pool type (0 = credit, 1 = main token, 2 = individual token).
    /// - `pool_identifier`: Optional 32-byte identifier for individual token pools.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, GroveTrunkQueryResult))`: The root hash and verified trunk result.
    /// - `Err(Error)`: If verification fails.
    pub fn verify_nullifiers_trunk_query(
        proof: &[u8],
        pool_type: u32,
        pool_identifier: Option<&[u8]>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, GroveTrunkQueryResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_nullifiers_trunk_query
        {
            0 => Self::verify_nullifiers_trunk_query_v0(
                proof,
                pool_type,
                pool_identifier,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_nullifiers_trunk_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
