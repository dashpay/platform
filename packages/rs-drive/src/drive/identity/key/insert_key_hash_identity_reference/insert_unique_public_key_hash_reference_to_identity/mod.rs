mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;

use crate::error::Error;

use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Generates a set of operations to insert a unique public key hash reference.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - A byte array representing the identity id.
    /// * `public_key_hash` - The hash of the public key.
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
    pub fn insert_unique_public_key_hash_reference_to_identity_operations(
        &self,
        identity_id: [u8; 32],
        public_key_hash: [u8; 20],
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
            .insert_unique_public_key_hash_reference_to_identity
        {
            0 => self.insert_unique_public_key_hash_reference_to_identity_operations_v0(
                identity_id,
                public_key_hash,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_unique_public_key_hash_reference_to_identity_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
