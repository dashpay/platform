mod v0;

use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use grovedb::reference_path::ReferencePathType;
use dpp::identity::{IdentityPublicKey, Purpose, SecurityLevel};
use dpp::version::drive_versions::DriveVersion;
use crate::drive::Drive;
use crate::drive::flags::SINGLE_EPOCH_FLAGS_SIZE;
use crate::drive::grove_operations::BatchInsertTreeApplyType;
use crate::drive::identity::{identity_key_location_within_identity_vec, identity_query_keys_full_tree_path, identity_query_keys_purpose_tree_path};
use crate::drive::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;
use crate::drive::object_size_info::PathKeyInfo::PathFixedSizeKey;
use crate::drive::operation::LowLevelDriveOperation;
use crate::error::drive::DriveError;
use crate::error::Error;

impl Drive {
    /// Generates a vector of operations for inserting key searchable references.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - An array of bytes representing the identity id.
    /// * `identity_key` - A reference to the `IdentityPublicKey` struct.
    /// * `key_id_bytes` - The byte representation of the key id.
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to an optional `HashMap` for estimated layer information.
    /// * `transaction` - The `TransactionArg` argument.
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
    pub fn insert_key_searchable_references_operations(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        key_id_bytes: &[u8],
        estimated_costs_only_with_layer_info: &mut Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.methods.identity.keys.insert.insert_key_searchable_references {
            0 => self.insert_key_searchable_references_operations_v0(identity_id, identity_key, key_id_bytes, estimated_costs_only_with_layer_info, transaction, drive_operations, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_key_searchable_references_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}