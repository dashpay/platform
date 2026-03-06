mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

/// Result type for verified nullifier changes per block
pub type VerifiedNullifierChangesPerBlock = Vec<(u64, Vec<[u8; 32]>)>;

impl Drive {
    /// Verifies the proof of recent nullifier changes starting from a given block height.
    ///
    /// This method validates and extracts nullifier changes from the provided proof.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the cryptographic proof for the nullifier changes.
    /// - `start_block_height`: The block height to start verifying from.
    /// - `limit`: Optional maximum number of blocks to verify.
    /// - `verify_subset_of_proof`: A boolean flag indicating whether to verify only a subset of the proof.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, VerifiedNullifierChangesPerBlock))`: On success, returns:
    ///   - `RootHash`: The root hash of the Merkle tree.
    ///   - `VerifiedNullifierChangesPerBlock`: Vector of (block_height, nullifiers) tuples.
    /// - `Err(Error)`: If verification fails.
    pub fn verify_recent_nullifier_changes(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedNullifierChangesPerBlock), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_recent_nullifier_changes
        {
            0 => Self::verify_recent_nullifier_changes_v0(
                proof,
                start_block_height,
                limit,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_recent_nullifier_changes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
