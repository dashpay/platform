mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof for shielded nullifier spend status.
    pub fn verify_shielded_nullifiers(
        proof: &[u8],
        nullifiers: &[Vec<u8>],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, bool)>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_shielded_nullifiers
        {
            0 => Self::verify_shielded_nullifiers_v0(
                proof,
                nullifiers,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_shielded_nullifiers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
