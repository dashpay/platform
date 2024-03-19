use crate::drive::identity::{
    identity_query_keys_purpose_tree_path, identity_query_keys_tree_path,
};
use crate::drive::object_size_info::DriveKeyInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::{Purpose, SecurityLevel};
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// This creates the key query tree structure operations and adds them to the
    /// mutable drive_operations vector
    pub(super) fn create_new_identity_key_query_trees_operations_v0(
        &self,
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let identity_query_key_tree = identity_query_keys_tree_path(identity_id.as_slice());

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_root_key_reference_tree(
                identity_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;
        }

        // There are 4 Purposes: Authentication, Encryption, Decryption, Transfer
        for purpose in Purpose::authentication_and_transfer() {
            self.batch_insert_empty_tree(
                identity_query_key_tree,
                DriveKeyInfo::Key(vec![purpose as u8]),
                None,
                drive_operations,
                drive_version,
            )?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Drive::add_estimation_costs_for_purpose_in_key_reference_tree(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                    purpose,
                    drive_version,
                )?;
            }
        }
        // There are 4 Security Levels: Master, Critical, High, Medium
        // For the Authentication Purpose we insert every tree
        let identity_key_authentication_tree = identity_query_keys_purpose_tree_path(
            identity_id.as_slice(),
            &[Purpose::AUTHENTICATION as u8],
        );
        // TODO: We probably don't need to create all security levels because DPP
        //  requires only Master and High and we don't want to do identity more expensive
        //  than it's needed
        for security_level in SecurityLevel::full_range() {
            self.batch_insert_empty_tree(
                identity_key_authentication_tree,
                DriveKeyInfo::Key(vec![security_level as u8]),
                None,
                drive_operations,
                drive_version,
            )?;
            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Drive::add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                    security_level,
                    drive_version,
                )?;
            }
        }
        Ok(())
    }
}
