mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches identity ids from storage based on unique public key hashes.
    ///
    /// This function leverages the versioning system to direct the fetch operation to the appropriate handler based on the `DriveVersion` provided.
    ///
    /// # Arguments
    ///
    /// * `public_key_hashes` - A reference to a slice of unique public key hashes corresponding to the identity ids to be fetched.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `BTreeMap` mapping each public key hash to its corresponding identity id if it exists, otherwise an `Error` if the fetch operation fails or the version is not supported.
    pub fn fetch_identity_ids_by_unique_public_key_hashes(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 20], Option<[u8; 32]>>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .public_key_hashes
            .fetch_identity_ids_by_unique_public_key_hashes
        {
            0 => self.fetch_identity_ids_by_unique_public_key_hashes_v0(
                public_key_hashes,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_ids_by_unique_public_key_hashes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches identity ids from storage based on unique public key hashes. This function also logs drive operations.
    ///
    /// This function leverages the versioning system to direct the fetch operation to the appropriate handler based on the `DriveVersion` provided.
    ///
    /// # Arguments
    ///
    /// * `public_key_hashes` - A reference to a slice of unique public key hashes corresponding to the identity ids to be fetched.
    /// * `transaction` - Transaction arguments.
    /// * `drive_operations` - A mutable reference to a vector of drive operations.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `BTreeMap` mapping each public key hash to its corresponding identity id if it exists, otherwise an `Error` if the fetch operation fails or the version is not supported.
    pub(crate) fn fetch_identity_ids_by_unique_public_key_hashes_operations(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 20], Option<[u8; 32]>>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .public_key_hashes
            .fetch_identity_ids_by_unique_public_key_hashes
        {
            0 => self.fetch_identity_ids_by_unique_public_key_hashes_operations_v0(
                public_key_hashes,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_ids_by_unique_public_key_hashes_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
