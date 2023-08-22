mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Given public key hashes, fetches full identities as proofs.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `public_key_hashes` - The slice of public key hashes for which to fetch the identities.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of bytes representing the proved identities, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn prove_full_identities_by_unique_public_key_hashes(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .prove
            .prove_full_identities_by_unique_public_key_hashes
        {
            0 => self.prove_full_identities_by_unique_public_key_hashes_v0(
                public_key_hashes,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_full_identities_by_unique_public_key_hashes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
