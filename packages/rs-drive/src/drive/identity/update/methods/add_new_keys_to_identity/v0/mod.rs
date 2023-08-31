use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::IdentityPublicKey;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations for adding new keys to an identity
    pub fn add_new_keys_to_identity_operations_v0(
        &self,
        identity_id: [u8; 32],
        unique_keys_to_add: Vec<IdentityPublicKey>,
        non_unique_keys_to_add: Vec<IdentityPublicKey>,
        with_references: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        for key in unique_keys_to_add {
            self.insert_new_unique_key_operations(
                identity_id,
                key,
                with_references,
                estimated_costs_only_with_layer_info,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
        }

        for key in non_unique_keys_to_add {
            self.insert_new_non_unique_key_operations(
                identity_id,
                key,
                with_references,
                estimated_costs_only_with_layer_info,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
        }
        Ok(drive_operations)
    }
}
