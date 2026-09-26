mod v0;

use crate::drive::shielded::paths::token_shielded_pool_latest_recorded_anchor_path_query;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof for the most recent shielded anchor.
    pub fn verify_most_recent_shielded_anchor(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_most_recent_shielded_anchor
        {
            0 => Self::verify_most_recent_shielded_anchor_v0(
                proof,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_most_recent_shielded_anchor".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl Drive {
    /// Verifies a proof of a TOKEN shielded pool's most recently recorded anchor. Same
    /// versioning as [`Drive::verify_most_recent_shielded_anchor`].
    pub fn verify_most_recent_token_shielded_pool_anchor(
        proof: &[u8],
        token_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_most_recent_shielded_anchor
        {
            0 => Self::verify_pool_most_recent_anchor_v0(
                proof,
                token_shielded_pool_latest_recorded_anchor_path_query(token_id),
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_most_recent_token_shielded_pool_anchor".to_string(),
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
    fn test_verify_most_recent_shielded_anchor_unknown_version_mismatch() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_most_recent_shielded_anchor = 255;

        let result = Drive::verify_most_recent_shielded_anchor(&[], false, &platform_version);

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
