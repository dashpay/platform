mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fee::op::LowLevelDriveOperation;
use crate::query::QueryResultEncoding;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches an identity with all its information from storage and then encodes the results with a specified encoding.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `public_key_hash` - Public key hash of the identity to fetch.
    /// * `encoding` - Encoding to use for the returned value.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a byte vector of the encoded identity, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn fetch_serialized_full_identity_by_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        encoding: QueryResultEncoding,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version
            .methods
            .identity
            .fetch
            .public_key_hashes
            .fetch_serialized_full_identity_by_unique_public_key_hash
        {
            0 => self.fetch_serialized_full_identity_by_unique_public_key_hash_v0(
                public_key_hash,
                encoding,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_serialized_full_identity_by_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
