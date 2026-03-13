mod v0;

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
    /// and stores cmx || nullifier || encrypted_note as the item value.
    ///
    /// # Parameters
    /// - `nullifier`: The 32-byte nullifier (rho) of the spent note in this action
    /// - `cmx`: The 32-byte note commitment
    /// - `encrypted_note`: The encrypted note payload (216 bytes)
    /// - `platform_version`: The platform version for dispatch
    pub fn insert_note_op(
        nullifier: [u8; 32],
        cmx: [u8; 32],
        encrypted_note: Vec<u8>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.shielded.insert_note {
            0 => Self::insert_note_op_v0(nullifier, cmx, encrypted_note),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_note_op".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
