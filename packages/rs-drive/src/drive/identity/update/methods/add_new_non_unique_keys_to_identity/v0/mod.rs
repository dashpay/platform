use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::block::block_info::BlockInfo;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::fee::calculate_fee;
use dpp::state_transition::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// Add new non unique keys to an identity
    pub fn add_new_non_unique_keys_to_identity_v0(
        &self,
        identity_id: [u8; 32],
        keys_to_add: Vec<IdentityPublicKey>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.add_new_keys_to_identity_operations(
            identity_id,
            vec![],
            keys_to_add,
            true,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            drive_version,
        )?;
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            drive_version
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }
}
