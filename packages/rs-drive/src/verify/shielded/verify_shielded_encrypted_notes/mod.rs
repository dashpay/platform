mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

/// A single shielded note item recovered from a verified
/// `GetShieldedEncryptedNotes` proof: the raw bytes copied out of the stored
/// `cmx || nullifier || cv_net || encrypted_note` commitment-tree item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedShieldedEncryptedNote {
    /// 32-byte note commitment.
    pub cmx: Vec<u8>,
    /// 32-byte nullifier (the output note's rho).
    pub nullifier: Vec<u8>,
    /// 32-byte Orchard value commitment (used for OVK outgoing-note recovery).
    pub cv_net: Vec<u8>,
    /// Encrypted note ciphertext (`epk || enc_ciphertext || out_ciphertext`).
    pub encrypted_note: Vec<u8>,
}

impl Drive {
    /// Verifies a proof for shielded encrypted notes.
    ///
    /// Returns `(root_hash, notes, total_count)`. `total_count` is the
    /// on-chain total number of notes in the shielded `CommitmentTree`,
    /// extracted from the SAME proof (the parent CommitmentTree element is
    /// always present in a note-fetch proof) — wallets get the sync
    /// progress-bar denominator for free on every chunk fetch.
    pub fn verify_shielded_encrypted_notes(
        proof: &[u8],
        start_index: u64,
        count: u32,
        max_elements: u32,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<VerifiedShieldedEncryptedNote>, u64), Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_verify_shielded_encrypted_notes_unknown_version_mismatch() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .shielded
            .verify_shielded_encrypted_notes = 255;

        let result = Drive::verify_shielded_encrypted_notes(&[], 0, 0, 0, false, &platform_version);

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
