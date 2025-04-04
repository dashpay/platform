use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_burn_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        burn_amount: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.token_burn_add_to_operations_v0(
            token_id,
            identity_id,
            burn_amount,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;

        Ok(fees)
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_burn_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        burn_amount: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.token_burn_operations_v0(
            token_id,
            identity_id,
            burn_amount,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    pub(super) fn token_burn_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        burn_amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        drive_operations.extend(self.remove_from_identity_token_balance_operations(
            token_id,
            identity_id,
            burn_amount,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        drive_operations.extend(self.remove_from_token_total_supply_operations(
            token_id,
            burn_amount,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        Ok(drive_operations)
    }
}
