use crate::drive::flags::SINGLE_EPOCH_FLAGS_SIZE;
use crate::drive::grove_operations::BatchInsertTreeApplyType;
use crate::drive::identity::{
    identity_key_location_within_identity_vec,
    identity_query_keys_for_authentication_full_tree_path,
    identity_query_keys_for_transfer_full_tree_path, identity_query_keys_purpose_tree_path,
};
use crate::drive::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;
use crate::drive::object_size_info::PathKeyInfo::PathFixedSizeKey;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, Purpose, SecurityLevel};
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Generates a vector of operations for inserting key searchable references.
    pub(super) fn insert_key_searchable_references_operations_v0(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        key_id_bytes: &[u8],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let purpose = identity_key.purpose();
        let security_level = identity_key.security_level();
        let purpose_vec = vec![purpose as u8];
        let security_level_vec = vec![security_level as u8];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_root_key_reference_tree(
                identity_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;

            Self::add_estimation_costs_for_purpose_in_key_reference_tree(
                identity_id,
                estimated_costs_only_with_layer_info,
                purpose,
                drive_version,
            )?;

            if matches!(purpose, Purpose::AUTHENTICATION) {
                Self::add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                    security_level,
                    drive_version,
                )?;
            }
        }

        // Now lets add in references so we can query keys.
        // We assume the following, the identity already has a the basic Query Tree

        match purpose {
            Purpose::AUTHENTICATION => {
                if security_level != SecurityLevel::MEDIUM {
                    // Not Medium (Medium is already pre-inserted)

                    let purpose_path = identity_query_keys_purpose_tree_path(
                        identity_id.as_slice(),
                        purpose_vec.as_slice(),
                    );

                    let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                        BatchInsertTreeApplyType::StatefulBatchInsertTree
                    } else {
                        BatchInsertTreeApplyType::StatelessBatchInsertTree {
                            in_tree_using_sums: false,
                            is_sum_tree: false,
                            flags_len: SINGLE_EPOCH_FLAGS_SIZE,
                        }
                    };

                    // We need to insert the security level if it doesn't yet exist
                    self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
                        PathFixedSizeKey((purpose_path, vec![security_level as u8])),
                        None,
                        apply_type,
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;
                }

                // Now let's set the reference
                let reference_path = identity_query_keys_for_authentication_full_tree_path(
                    identity_id.as_slice(),
                    security_level_vec.as_slice(),
                );

                let key_reference = identity_key_location_within_identity_vec(key_id_bytes);
                self.batch_insert(
                    PathFixedSizeKeyRefElement((
                        reference_path,
                        key_id_bytes,
                        Element::new_reference_with_flags(
                            ReferencePathType::UpstreamRootHeightReference(2, key_reference),
                            None,
                        ),
                    )),
                    drive_operations,
                    drive_version,
                )
            }
            Purpose::TRANSFER => {
                // Now let's set the reference
                let reference_path =
                    identity_query_keys_for_transfer_full_tree_path(identity_id.as_slice());
                let key_reference = identity_key_location_within_identity_vec(key_id_bytes);
                self.batch_insert(
                    PathFixedSizeKeyRefElement((
                        reference_path,
                        key_id_bytes,
                        Element::new_reference_with_flags(
                            ReferencePathType::UpstreamRootHeightReference(2, key_reference),
                            None,
                        ),
                    )),
                    drive_operations,
                    drive_version,
                )
            }
            _ => Ok(()),
        }
    }
}
