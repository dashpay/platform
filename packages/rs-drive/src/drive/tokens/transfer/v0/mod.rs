// token_transfer/v0.rs
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
    pub(super) fn token_transfer_v0(
        &self,
        token_id: [u8; 32],
        from_identity_id: [u8; 32],
        to_identity_id: [u8; 32],
        amount: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.token_transfer_add_to_operations_v0(
            token_id,
            from_identity_id,
            to_identity_id,
            amount,
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

    pub(super) fn token_transfer_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        from_identity_id: [u8; 32],
        to_identity_id: [u8; 32],
        amount: u64,
        apply: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.token_transfer_operations_v0(
            token_id,
            from_identity_id,
            to_identity_id,
            amount,
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

    pub(super) fn token_transfer_operations_v0(
        &self,
        _token_id: [u8; 32],
        from_identity_id: [u8; 32],
        to_identity_id: [u8; 32],
        amount: u64,
        _previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
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
                from_identity_id,
                esti,
                &platform_version.drive,
            )?;
            Self::add_estimation_costs_for_negative_credit(
                to_identity_id,
                esti,
                &platform_version.drive,
            )?;
        }

        // Fetch sender balance
        let from_balance = self
            .fetch_identity_balance_operations(
                from_identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "sender identity must have a balance".to_string(),
            )))?;

        if from_balance < amount {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "sender does not have enough balance to transfer".to_string(),
            )));
        }

        let new_from_balance = from_balance - amount;
        drive_operations
            .push(self.update_identity_balance_operation_v0(from_identity_id, new_from_balance)?);

        // Fetch recipient balance
        let to_balance = self
            .fetch_identity_balance_operations(
                to_identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .unwrap_or(0);

        let new_to_balance =
            to_balance
                .checked_add(amount)
                .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                    "overflow on recipient balance".to_string(),
                )))?;
        drive_operations
            .push(self.update_identity_balance_operation_v0(to_identity_id, new_to_balance)?);

        // Total supply remains the same.
        Ok(drive_operations)
    }
}
