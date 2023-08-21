use crate::drive::identity::identity_path;
use crate::drive::identity::IdentityRootStructure::{IdentityTreeKeyReferences, IdentityTreeKeys};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::IdentityPublicKey;

use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    pub(super) fn create_key_tree_with_keys_operations_v0(
        &self,
        identity_id: [u8; 32],
        keys: Vec<IdentityPublicKey>,
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

        let (unique_keys, non_unique_keys): (Vec<IdentityPublicKey>, Vec<IdentityPublicKey>) = keys
            .into_iter()
            .partition(|key| key.key_type().is_unique_key_type());

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
