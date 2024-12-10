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
            &mut None,
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

    pub(super) fn token_burn_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        burn_amount: u64,
        apply: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
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
            previous_batch_operations,
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
        _previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        // Add estimation info if needed
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
            .fetch_identity_balance_operations(
                identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "there should be a balance when burning tokens".to_string(),
            )))?;

        if current_balance < burn_amount {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "cannot burn more tokens than currently owned".to_string(),
            )));
        }

        let new_balance = current_balance - burn_amount;

        // Update identity balance
        drive_operations.push(self.update_identity_balance_operation_v0(identity_id, new_balance)?);

        // Update total supply for the token (subtract burn_amount)
        let current_supply = self
            .fetch_token_total_supply_operations(
                token_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "token should have a total supply".to_string(),
            )))?;

        if current_supply < burn_amount {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "cannot burn more tokens than total supply".to_string(),
            )));
        }

        let new_supply = current_supply - burn_amount;
        drive_operations.push(self.update_token_total_supply_operation_v0(token_id, new_supply)?);

        Ok(drive_operations)
    }
}
