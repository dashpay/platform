use dpp::block::block_info::BlockInfo;

use crate::drive::Drive;

use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use grovedb::batch::KeyInfoPath;

use dpp::fee::fee_result::FeeResult;

use dpp::prelude::Revision;

use dpp::version::PlatformVersion;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use std::collections::HashMap;

impl Drive {
    //todo: this should probably not exist
    /// Update revision for specific identity
    #[inline(always)]
    pub(super) fn update_identity_revision_v0(
        &self,
        identity_id: [u8; 32],
        revision: Revision,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        // TODO: In case of dry run we will get less because we replace the same bytes

        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = vec![self.update_identity_revision_operation_v0(
            identity_id,
            revision,
            &mut estimated_costs_only_with_layer_info,
            platform_version,
        )?];

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
        )?;

        Ok(fees)
    }
}
