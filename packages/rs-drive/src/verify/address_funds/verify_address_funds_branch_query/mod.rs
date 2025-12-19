//! Module for verifying address funds branch chunk queries.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::GroveBranchQueryResult;
use grovedb_merk::CryptoHash;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies a branch chunk proof for the address funds tree.
    ///
    /// This method validates a branch chunk proof and returns the verified result
    /// containing the elements and leaf keys for subsequent branch queries.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the serialized branch chunk proof.
    /// - `key`: The key that was navigated to in the tree.
    /// - `depth`: The depth of the branch that was returned.
    /// - `expected_root_hash`: The expected root hash from the parent trunk/branch proof's leaf info.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    /// - `Ok(GroveBranchQueryResult)`: On success, returns the verified branch query result
    ///   containing elements and leaf keys.
    /// - `Err(Error)`: If verification fails.
    ///
    /// # Errors
    /// - [`Error::Proof`]: If the proof is invalid or corrupted.
    /// - [`Error::Drive(DriveError::UnknownVersionMismatch)`]: If called with an unsupported platform version.
    /// - [`Error::Drive(DriveError::InvalidInput)`]: If the depth is outside the allowed range.
    pub fn verify_address_funds_branch_query(
        proof: &[u8],
        key: Vec<u8>,
        depth: u8,
        expected_root_hash: CryptoHash,
        platform_version: &PlatformVersion,
    ) -> Result<GroveBranchQueryResult, Error> {
        match platform_version
            .drive
            .methods
            .verify
            .address_funds
            .verify_address_funds_branch_query
        {
            0 => Self::verify_address_funds_branch_query_v0(
                proof,
                key,
                depth,
                expected_root_hash,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_address_funds_branch_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
