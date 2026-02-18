mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof for shielded encrypted notes.
    pub fn verify_shielded_encrypted_notes(
        proof: &[u8],
        start_index: u64,
        count: u32,
        max_elements: u32,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, Vec<u8>, Vec<u8>)>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_shielded_encrypted_notes
        {
            0 => Self::verify_shielded_encrypted_notes_v0(
                proof,
                start_index,
                count,
                max_elements,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_shielded_encrypted_notes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
