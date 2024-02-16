use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;

use crate::error::Error;

use crate::fee::op::LowLevelDriveOperation;
use grovedb::batch::KeyInfoPath;

use dpp::prelude::IdentityNonce;

use dpp::version::PlatformVersion;
use grovedb::{Element, EstimatedLayerInformation};

use std::collections::HashMap;

impl Drive {
    /// Update the nonce of the identity
    /// Nonces get bumped on all identity state transitions except those that use an asset lock
    pub(in crate::drive::identity::update) fn update_identity_nonce_operation_v0(
        &self,
        identity_id: [u8; 32],
        nonce: IdentityNonce,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<LowLevelDriveOperation, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_update_nonce(
                identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let identity_path = identity_path_vec(identity_id.as_slice());
        let nonce_bytes = nonce.to_be_bytes().to_vec();

        Ok(LowLevelDriveOperation::replace_for_known_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeNonce).to_vec(),
            Element::new_item(nonce_bytes),
        ))
    }
}
