use crate::drive::identity::{
    identity_key_location_within_identity_vec, identity_query_keys_purpose_tree_path_vec,
    identity_query_keys_security_level_tree_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, Purpose};
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Refreshes identity key reference operations.
    pub fn refresh_identity_key_reference_operations_v0(
        &self,
        identity_id: [u8; 32],
        key: &IdentityPublicKey,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
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

        if key.contract_bounds().is_some() {
            // if there are contract bounds we need to insert them
            self.refresh_potential_contract_info_key_references(
                identity_id,
                &key,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            )?;
        }

        Ok(())
    }
}
