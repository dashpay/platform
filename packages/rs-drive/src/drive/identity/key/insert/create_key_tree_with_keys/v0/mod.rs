use crate::drive::identity::identity_path;
use crate::drive::identity::IdentityRootStructure::{IdentityTreeKeyReferences, IdentityTreeKeys};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::IdentityPublicKey;

use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Creates a key tree and associates the provided keys with it (version 0).
    ///
    /// This function constructs a key tree for a specific identity ID and associates the
    /// provided public keys with it. The tree creation and association are performed as
    /// low-level drive operations.
    ///
    /// # Parameters
    ///
    /// * `identity_id`: A 32-byte array representing the identity ID.
    /// * `keys`: A vector of `IdentityPublicKey` to be associated with the identity.
    /// * `estimated_costs_only_with_layer_info`: An optional mutable reference to a `HashMap`
    ///   that stores estimated layer information based on the key information path.
    /// * `transaction`: A `TransactionArg` object representing the context of the current transaction.
    /// * `platform_version`: A reference to the `PlatformVersion` struct, providing versioning details.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `LowLevelDriveOperation`, which represents the
    /// operations needed to perform the tree creation and key association, or an `Error` if
    /// any step fails.
    ///
    /// # Notes
    ///
    /// - This function divides the provided keys into unique and non-unique types, and processes
    ///   them accordingly.
    /// - Updates to the estimated costs (if provided) are made based on the version of the drive.
    /// ```
    pub(super) fn create_key_tree_with_keys_operations_v0(
        &self,
        identity_id: [u8; 32],
        keys: Vec<IdentityPublicKey>,
        register_all_keys_as_non_unique_for_masternode: bool,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let drive_version = &platform_version.drive;
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;
        }
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        let identity_path = identity_path(identity_id.as_slice());
        self.batch_insert_empty_tree(
            identity_path,
            IdentityTreeKeys.to_drive_key_info(),
            None,
            &mut batch_operations,
            drive_version,
        )?;

        self.batch_insert_empty_tree(
            identity_path,
            IdentityTreeKeyReferences.to_drive_key_info(),
            None,
            &mut batch_operations,
            drive_version,
        )?;

        // We create the query trees structure
        self.create_new_identity_key_query_trees_operations(
            identity_id,
            estimated_costs_only_with_layer_info,
            &mut batch_operations,
            drive_version,
        )?;

        let (unique_keys, non_unique_keys): (Vec<IdentityPublicKey>, Vec<IdentityPublicKey>) =
            if register_all_keys_as_non_unique_for_masternode {
                (vec![], keys)
            } else {
                keys.into_iter()
                    .partition(|key| key.key_type().is_unique_key_type())
            };

        for key in unique_keys.into_iter() {
            self.insert_new_unique_key_operations(
                identity_id,
                key,
                true,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                &mut batch_operations,
                platform_version,
            )?;
        }

        for key in non_unique_keys.into_iter() {
            self.insert_new_non_unique_key_operations(
                identity_id,
                key,
                // if we are a masternode this then we should have false here
                !register_all_keys_as_non_unique_for_masternode,
                true,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                &mut batch_operations,
                platform_version,
            )?;
        }
        Ok(batch_operations)
    }
}
