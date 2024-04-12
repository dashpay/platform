mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::identity::Purpose;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves identities contract keys given identity ids and contract id.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_ids` - The slice of identity ids to prove
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of bytes representing the proved identities, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn prove_identities_contract_keys(
        &self,
        identity_ids: &[[u8; 32]],
        contract_id: &[u8; 32],
        purposes: Vec<Purpose>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version
            .methods
            .identity
            .prove
            .identities_contract_keys
        {
            0 => self.prove_identities_contract_keys_v0(
                identity_ids,
                contract_id,
                purposes,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_identities_contract_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
