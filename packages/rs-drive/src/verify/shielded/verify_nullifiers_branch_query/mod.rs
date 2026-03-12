//! Module for verifying nullifiers branch chunk queries.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::GroveBranchQueryResult;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies a branch chunk proof for the nullifiers tree of a shielded pool.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the serialized branch chunk proof.
    /// - `pool_type`: The shielded pool type (0 = credit, 1 = main token, 2 = individual token).
    /// - `pool_identifier`: Optional 32-byte identifier for individual token pools.
    /// - `key`: The key that was navigated to in the tree.
    /// - `depth`: The depth of the branch that was returned.
    /// - `expected_root_hash`: The expected root hash from the parent trunk/branch proof.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    /// - `Ok(GroveBranchQueryResult)`: The verified branch query result.
    /// - `Err(Error)`: If verification fails.
    pub fn verify_nullifiers_branch_query(
        proof: &[u8],
        pool_type: u32,
        pool_identifier: Option<&[u8]>,
        key: Vec<u8>,
        depth: u8,
        expected_root_hash: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<GroveBranchQueryResult, Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_nullifiers_branch_query
        {
            0 => Self::verify_nullifiers_branch_query_v0(
                proof,
                pool_type,
                pool_identifier,
                key,
                depth,
                expected_root_hash,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_nullifiers_branch_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_verify_nullifiers_branch_query_unknown_version_mismatch() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_nullifiers_branch_query = 255;

        let result = Drive::verify_nullifiers_branch_query(
            &[],
            0,
            None,
            vec![],
            0,
            [0u8; 32],
            &platform_version,
        );

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
