mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use grovedb::TransactionArg;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Fetches the Identity's contract document nonce from the backing store
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - Identity Id to prove.
    /// * `contract_id` - For Contract Id to prove.
    /// * `transaction` - Transaction arguments.
    /// * `platform_version` - A reference to the platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `Option` for the Identity's revision, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn prove_identity_contract_nonce(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version.methods.identity.prove.identity_contract_nonce {
            0 => self.prove_identity_contract_nonce_v0(
                identity_id,
                contract_id,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_identity_contract_nonce".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
