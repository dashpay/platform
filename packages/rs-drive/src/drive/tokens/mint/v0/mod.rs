use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    pub(super) fn token_mint_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        issuance_amount: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.token_mint_add_to_operations_v0(
            token_id,
            identity_id,
            issuance_amount,
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

    pub(super) fn token_mint_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        issuance_amount: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.token_mint_operations_v0(
            token_id,
            identity_id,
            issuance_amount,
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

    pub(super) fn token_mint_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        issuance_amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        // Estimation
        if let Some(esti) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(esti, &platform_version.drive)?;
            Self::add_estimation_costs_for_negative_credit(
                identity_id,
                esti,
                &platform_version.drive,
            )?;
        }

        // Fetch current balance
        let current_balance = self
            .fetch_identity_token_balance_operations(
                token_id,
                identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .unwrap_or(0);

        let new_balance = current_balance
            .checked_add(issuance_amount)
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "overflow when adding issuance_amount".to_string(),
            )))?;

        // Update identity balance
        drive_operations.push(self.update_identity_balance_operation_v0(identity_id, new_balance)?);

        drive_operations.push(self.add_to_token_total_supply_operations(
            token_id,
            issuance_amount,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        Ok(drive_operations)
    }
}
