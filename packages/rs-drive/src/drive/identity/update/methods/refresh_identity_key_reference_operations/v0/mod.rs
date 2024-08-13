use crate::drive::identity::{
    identity_contract_info_group_keys_path_vec, identity_contract_info_group_path_key_purpose_vec,
    identity_contract_info_root_path_vec, identity_key_location_within_identity_vec,
    identity_key_path_vec, identity_query_keys_for_authentication_full_tree_path,
    identity_query_keys_for_direct_searchable_reference_full_tree_path,
    identity_query_keys_purpose_tree_path, identity_query_keys_purpose_tree_path_vec,
    identity_query_keys_security_level_tree_path_vec, identity_query_keys_tree_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, Purpose};
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::reference_path::ReferencePathType::SiblingReference;
use grovedb::{Element, EstimatedLayerInformation};
use integer_encoding::VarInt;
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Updates the revision for a specific identity. This function is version controlled.
    pub fn refresh_identity_key_reference_operations_v0(
        &self,
        identity_id: [u8; 32],
        key: &IdentityPublicKey,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // At this point, we need to refresh reference to that Identity key that was just updated (disable is an update)

        let key_id_bytes = key.id().encode_var_vec();

        let key_reference = identity_key_location_within_identity_vec(&key_id_bytes);

        let identity_key_reference = Element::new_reference_with_flags(
            ReferencePathType::UpstreamRootHeightReference(2, key_reference),
            None,
        );

        let trust_refresh_reference = true; // todo: check if this needs to be false

        let purpose = key.purpose();
        let security_level = key.security_level();

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_purpose_in_key_reference_tree(
                identity_id,
                estimated_costs_only_with_layer_info,
                key.purpose(),
                &platform_version.drive,
            )?;

            if matches!(purpose, Purpose::AUTHENTICATION) {
                Self::add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                    key.security_level(),
                    &platform_version.drive,
                )?;
            }
        }

        let key_path_for_refresh = match purpose {
            Purpose::AUTHENTICATION => {
                // Now let's set the reference
                Some(identity_query_keys_security_level_tree_path_vec(
                    identity_id.as_slice(),
                    security_level,
                ))
            }
            Purpose::TRANSFER | Purpose::VOTING => {
                // Now let's set the reference
                Some(identity_query_keys_purpose_tree_path_vec(
                    identity_id.as_slice(),
                    purpose,
                ))
            }
            _ => None,
        };

        if let Some(key_path) = key_path_for_refresh {
            self.batch_refresh_reference(
                key_path,
                key_id_bytes.to_vec(),
                identity_key_reference.clone(),
                trust_refresh_reference,
                drive_operations,
                &platform_version.drive,
            )?;
        }

        if let Some(contract_info) = key.contract_bounds() {
            // 2) [Root ; <identity> ; Contract Info ; Contract Bound ; Keys]
            let mut index_contract_info_path = identity_contract_info_root_path_vec(&identity_id);
            index_contract_info_path.push(
                contract_info
                    .contract_bounds_type_string()
                    .as_bytes()
                    .to_vec(),
            ); // todo: Check if contract bound type string should be used in path?
            index_contract_info_path.push(vec![key.id() as u8]);

            self.batch_refresh_reference(
                index_contract_info_path,
                key_id_bytes.to_vec(),
                identity_key_reference.clone(),
                trust_refresh_reference,
                drive_operations,
                &platform_version.drive,
            )?;

            if let Some(document_type) = contract_info.document_type() {
                let root_id = vec![]; // todo: really not sure about this vec![]. IdentityDataContractKeyApplyInfo has a root_id() method but not ContractBounds

                let mut contract_id_bytes_with_document_type_name = root_id.clone();
                contract_id_bytes_with_document_type_name.extend(document_type.as_bytes());
                let sibling_ref_key_purpose_path =
                    identity_contract_info_group_path_key_purpose_vec(
                        &identity_id,
                        &contract_id_bytes_with_document_type_name,
                        key.purpose(),
                    );

                self.batch_refresh_reference(
                    sibling_ref_key_purpose_path,
                    key_id_bytes.to_vec(),
                    Element::Reference(SiblingReference(key_id_bytes.to_vec()), Some(2), None),
                    trust_refresh_reference,
                    drive_operations,
                    &platform_version.drive,
                )?;

                let sibling_ref_group_keys_path =
                    identity_contract_info_group_keys_path_vec(&identity_id, &root_id.clone());

                self.batch_refresh_reference(
                    sibling_ref_group_keys_path,
                    key_id_bytes.to_vec(),
                    Element::Reference(SiblingReference(key_id_bytes.to_vec()), Some(2), None),
                    trust_refresh_reference,
                    drive_operations,
                    &platform_version.drive,
                )?;
            }
        }

        Ok(())
    }
}
