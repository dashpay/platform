mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's nonce from the backing store
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - Identity Id to prove.
    /// * `apply` - If `true`, the changes are applied, otherwise only the cost is estimated.
    /// * `transaction` - Transaction arguments.
    /// * `platform_version` - A reference to the platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a Proof for the Identity's nonce, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn prove_identity_nonce(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.identity.prove.identity_nonce {
            0 => self.prove_identity_nonce_v0(identity_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_identity_nonce".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
