mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::IdentityPublicKey;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use std::collections::HashMap;

impl Drive {
    /// Generates a set of operations to insert a new non-unique key into an identity.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - An array of bytes representing the identity id.
    /// * `identity_key` - The `IdentityPublicKey` to be inserted.
    /// * `with_references` - A boolean value indicating whether to include references in the operations.
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to an optional `HashMap` that may contain estimated layer information.
    /// * `transaction` - The transaction arguments.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` objects.
    /// * `drive_version` - The version of the drive.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns unit (`()`). If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` if the operation creation process fails or if the drive version does not match any of the implemented method versions.
    pub fn insert_new_non_unique_key_operations(
        &self,
        identity_id: [u8; 32],
        identity_key: IdentityPublicKey,
        with_references: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .keys
            .insert
            .insert_new_non_unique_key
        {
            0 => self.insert_new_non_unique_key_operations_v0(
                identity_id,
                identity_key,
                with_references,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_new_non_unique_key_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
