use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use crate::fee::calculate_fee;
use dpp::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id
    pub fn add_to_identity_balance_v0(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
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

        let batch_operations = self.add_to_identity_balance_operations_v0(
            identity_id,
            added_balance,
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
            drive_version,
        )?;

        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;

        Ok(fees)
    }



    /// Balances are stored in the balance tree under the identity's id
    /// This gets operations based on apply flag (stateful vs stateless)
    pub fn add_to_identity_balance_operations_v0(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(estimated_costs_only_with_layer_info);
            Self::add_estimation_costs_for_negative_credit(
                identity_id,
                estimated_costs_only_with_layer_info,
            );
        }

        let previous_balance = self
            .fetch_identity_balance_operations(
                identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
                drive_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance",
            )))?;

        let AddToPreviousBalanceOutcome {
            balance_modified,
            negative_credit_balance_modified,
        } = self.add_to_previous_balance(
            identity_id,
            previous_balance,
            added_balance,
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        if let Some(new_balance) = balance_modified {
            drive_operations
                .push(self.update_identity_balance_operation(identity_id, new_balance, drive_version)?);
        }

        if let Some(new_negative_balance) = negative_credit_balance_modified {
            drive_operations.push(
                self.update_identity_negative_credit_operation(identity_id, new_negative_balance, drive_version),
            );
        }

        Ok(drive_operations)
    }
}