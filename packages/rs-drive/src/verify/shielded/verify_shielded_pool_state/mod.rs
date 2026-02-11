mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof for the shielded pool total balance.
    pub fn verify_shielded_pool_state(
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_shielded_pool_state
        {
            0 => Self::verify_shielded_pool_state_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_shielded_pool_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
