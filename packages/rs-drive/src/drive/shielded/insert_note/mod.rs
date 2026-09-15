mod v0;

use crate::drive::shielded::paths::token_shielded_pool_notes_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;

impl Drive {
    /// Constructs the low-level drive operations to insert a note into the
    /// shielded pool commitment tree.
    ///
    /// This appends the note commitment (cmx) to the commitment tree frontier
    /// and stores `cmx || nullifier || cv_net || encrypted_note` as the item value.
    ///
    /// # Parameters
    /// - `nullifier`: The 32-byte nullifier (rho) of the spent note in this action
    /// - `cmx`: The 32-byte note commitment
    /// - `cv_net`: The 32-byte value commitment, stored unencrypted for OVK recovery
    /// - `encrypted_note`: The encrypted note payload (216 bytes)
    /// - `platform_version`: The platform version for dispatch
    pub fn insert_note_op(
        nullifier: [u8; 32],
        cmx: [u8; 32],
        cv_net: [u8; 32],
        encrypted_note: Vec<u8>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.shielded.insert_note {
            0 => Self::insert_note_op_v0(nullifier, cmx, cv_net, encrypted_note),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_note_op".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl Drive {
    /// Constructs the low-level drive operations to insert a note into a TOKEN shielded pool's
    /// commitment tree. Same layout and versioning as [`Drive::insert_note_op`], re-rooted under
    /// the token's pool.
    pub fn insert_token_pool_note_op(
        token_id: [u8; 32],
        nullifier: [u8; 32],
        cmx: [u8; 32],
        cv_net: [u8; 32],
        encrypted_note: Vec<u8>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.shielded.insert_note {
            0 => Ok(Self::insert_note_op_in_pool_v0(
                token_shielded_pool_notes_path_vec(token_id),
                nullifier,
                cmx,
                cv_net,
                encrypted_note,
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_token_pool_note_op".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
