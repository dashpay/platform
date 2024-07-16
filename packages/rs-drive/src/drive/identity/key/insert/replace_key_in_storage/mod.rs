mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::IdentityPublicKey;

use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Generates a set of operations to replace a key in storage.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - A slice of bytes representing the identity id.
    /// * `identity_key` - The `IdentityPublicKey` to be replaced.
    /// * `key_id_bytes` - The identifier of the key in bytes.
    /// * `change_in_bytes` - The change in size, in bytes.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` objects.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns unit (`()`). If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` if the operation creation process fails or if the drive version does not match any of the implemented method versions.
    pub fn replace_key_in_storage_operations(
        &self,
        identity_id: &[u8],
        identity_key: &IdentityPublicKey,
        key_id_bytes: &[u8],
        change_in_bytes: i32,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .keys
            .insert
            .replace_key_in_storage
        {
            0 => self.replace_key_in_storage_operations_v0(
                identity_id,
                identity_key,
                key_id_bytes,
                change_in_bytes,
                drive_operations,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "replace_key_in_storage_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
