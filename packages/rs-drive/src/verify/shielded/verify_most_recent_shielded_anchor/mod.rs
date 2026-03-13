mod v0;

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
