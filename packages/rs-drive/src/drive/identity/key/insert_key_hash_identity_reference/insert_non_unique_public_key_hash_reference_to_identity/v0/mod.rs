use crate::drive::object_size_info::DriveKeyInfo::KeyRef;
use crate::drive::object_size_info::PathKeyElementInfo::PathKeyRefElement;

use crate::drive::{
    non_unique_key_hashes_sub_tree_path_vec, non_unique_key_hashes_tree_path, Drive,
};

use crate::error::identity::IdentityError;
use crate::error::Error;

use crate::fee::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;

use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Insert a non unique public key hash reference that contains an identity id
    /// Contrary to the name this is not a reference but an Item containing the identity
    /// identifier
    pub(super) fn insert_non_unique_public_key_hash_reference_to_identity_operations_v0(
        &self,
        identity_id: [u8; 32],
        public_key_hash: [u8; 20],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        let mut key_already_present_in_another_identity = false;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_insert_non_unique_public_key_hash_reference(
                public_key_hash,
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;
        } else {
            let already_exists_in_unique = self.has_unique_public_key_hash_operations(
                public_key_hash,
                transaction,
                &mut drive_operations,
                drive_version,
            )?;

            if already_exists_in_unique {
                return Err(Error::Identity(IdentityError::UniqueKeyAlreadyExists(
                    "the key already exists in the unique set while we are inserting it to the non unique set",
                )));
            }

            key_already_present_in_another_identity = self
                .has_non_unique_public_key_hash_operations(
                    public_key_hash,
                    transaction,
                    &mut drive_operations,
                    drive_version,
                )?;

            if key_already_present_in_another_identity {
                let already_exists_for_identity = self
                    .has_non_unique_public_key_hash_already_for_identity_operations(
                        public_key_hash,
                        identity_id,
                        transaction,
                        &mut drive_operations,
                        drive_version,
                    )?;

                if already_exists_for_identity {
                    return Err(Error::Identity(IdentityError::IdentityKeyAlreadyExists(
                        "the key already exists for this user",
                    )));
                }
            }
        }

        if !key_already_present_in_another_identity {
            let non_unique_key_hashes_path = non_unique_key_hashes_tree_path();
            // We insert the parent tree
            self.batch_insert_empty_tree(
                non_unique_key_hashes_path,
                KeyRef(public_key_hash.as_slice()),
                None,
                &mut drive_operations,
                drive_version,
            )?;
        }

        let non_unique_key_hashes_path = non_unique_key_hashes_sub_tree_path_vec(public_key_hash);

        // The value is empty here because the key already has the identity id
        self.batch_insert::<0>(
            PathKeyRefElement((
                non_unique_key_hashes_path,
                identity_id.as_slice(),
                Element::Item(vec![], None),
            )),
            &mut drive_operations,
            drive_version,
        )?;

        Ok(drive_operations)
    }
}
