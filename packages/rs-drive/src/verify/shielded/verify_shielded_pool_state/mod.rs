mod v0;

use crate::drive::shielded::paths::token_shielded_pool_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof for the shielded pool total balance.
    pub fn verify_shielded_pool_state(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_shielded_pool_state
        {
            0 => {
                Self::verify_shielded_pool_state_v0(proof, verify_subset_of_proof, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_shielded_pool_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl Drive {
    /// Verifies a proof for a TOKEN shielded pool's total balance (the amount of the token
    /// currently shielded). Same versioning as [`Drive::verify_shielded_pool_state`].
    pub fn verify_token_shielded_pool_state(
        proof: &[u8],
        token_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_shielded_pool_state
        {
            0 => Self::verify_pool_state_v0(
                proof,
                token_shielded_pool_path_vec(token_id),
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_token_shielded_pool_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_verify_shielded_pool_state_unknown_version_mismatch() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_shielded_pool_state = 255;

        let result = Drive::verify_shielded_pool_state(&[], false, &platform_version);

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::UnknownVersionMismatch { .. }))
            ),
            "expected UnknownVersionMismatch, got {:?}",
            result,
        );
    }
}
