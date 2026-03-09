mod v0;

use crate::drive::shielded::nullifiers::types::NullifierChangePerBlock;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the proof of recent nullifier changes starting from a given block height.
    pub fn verify_recent_nullifier_changes(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<NullifierChangePerBlock>), Error> {
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
