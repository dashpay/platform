mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves identities with all their information from the given identity ids.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_ids` - The slice of identity ids to prove.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of bytes representing the proved identities, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn prove_full_identities(
        &self,
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version.methods.identity.prove.full_identities {
            0 => self.prove_full_identities_v0(identity_ids, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_full_identities".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
