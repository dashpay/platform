mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;

use crate::error::Error;

use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::IdentityPublicKey;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Generates a set of operations to insert a reference to a unique key.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - A byte array representing the identity id.
    /// * `identity_key` - A `IdentityPublicKey` object that needs to be hashed.
    /// * `estimated_costs_only_with_layer_info` - An optional mutable reference to a map from `KeyInfoPath` to `EstimatedLayerInformation`.
    /// * `transaction` - A `TransactionArg` to be used in the operation.
    /// * `drive_version` - A `DriveVersion` object.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - If successful, returns a vector of `LowLevelDriveOperation` objects. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` if the operation creation process fails or if the drive version does not match any of the implemented method versions.
    pub fn insert_reference_to_unique_key_operations(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match drive_version
            .methods
            .identity
            .keys
            .insert_key_hash_identity_reference
            .insert_reference_to_unique_key
        {
            0 => self.insert_reference_to_unique_key_operations_v0(
                identity_id,
                identity_key,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_reference_to_unique_key_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
