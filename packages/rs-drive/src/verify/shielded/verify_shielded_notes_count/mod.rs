mod v0;

use crate::drive::shielded::paths::token_shielded_pool_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof for the shielded notes commitment-tree leaf count.
    ///
    /// Returns the verified `(root_hash, Option<total_count>)`. The count
    /// is the first field of the proved `CommitmentTree` element, whose
    /// serialized bytes are bound into the Merk value hash — see
    /// `verify_shielded_notes_count_v0`.
    pub fn verify_shielded_notes_count(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_shielded_notes_count
        {
            0 => Self::verify_shielded_notes_count_v0(
                proof,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_shielded_notes_count".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl Drive {
    /// Verifies a proof of a TOKEN shielded pool's notes count. Same versioning as
    /// [`Drive::verify_shielded_notes_count`].
    pub fn verify_token_shielded_pool_notes_count(
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
            .verify_shielded_notes_count
        {
            0 => Self::verify_pool_notes_count_v0(
                proof,
                token_shielded_pool_path_vec(token_id),
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_token_shielded_pool_notes_count".to_string(),
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
    fn test_verify_shielded_notes_count_unknown_version_mismatch() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_shielded_notes_count = 255;

        let result = Drive::verify_shielded_notes_count(&[], false, &platform_version);

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
