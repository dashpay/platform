mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

/// Result type for verified compacted nullifier changes
/// Each entry is (start_block, end_block, nullifiers)
pub type VerifiedCompactedNullifierChanges = Vec<(u64, u64, Vec<[u8; 32]>)>;

impl Drive {
    /// Verifies the proof of compacted nullifier changes starting from a given block height.
    ///
    /// This method validates and extracts compacted nullifier changes from the provided proof.
    /// Compacted entries represent concatenated data from multiple blocks.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the cryptographic proof for the compacted nullifier changes.
    /// - `start_block_height`: The block height to start verifying from.
    /// - `limit`: Optional maximum number of compacted entries to verify.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, VerifiedCompactedNullifierChanges))`: On success, returns:
    ///   - `RootHash`: The root hash of the Merkle tree.
    ///   - `VerifiedCompactedNullifierChanges`: Vector of (start_block, end_block, nullifiers) tuples.
    /// - `Err(Error)`: If verification fails.
    pub fn verify_compacted_nullifier_changes(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedCompactedNullifierChanges), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_compacted_nullifier_changes
        {
            0 => Self::verify_compacted_nullifier_changes_v0(
                proof,
                start_block_height,
                limit,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_compacted_nullifier_changes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
