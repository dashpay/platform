mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use dpp::identity::Identity;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches identities with all their related information from storage based on unique public key hashes.
    ///
    /// This function leverages the versioning system to direct the fetch operation to the appropriate handler based on the `DriveVersion` provided.
    ///
    /// # Arguments
    ///
    /// * `public_key_hashes` - A slice of unique public key hashes corresponding to the identities to be fetched.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `BTreeMap` that maps each public key hash to the corresponding `Identity` (if it exists),
    /// otherwise an `Error` if the fetch operation fails or the version is not supported.
    pub fn fetch_full_identities_by_unique_public_key_hashes(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 20], Option<Identity>>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .public_key_hashes
            .fetch_full_identities_by_unique_public_key_hashes
        {
            0 => self.fetch_full_identities_by_unique_public_key_hashes_v0(
                public_key_hashes,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_full_identities_by_unique_public_key_hashes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
