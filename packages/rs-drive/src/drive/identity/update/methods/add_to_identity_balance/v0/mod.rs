use crate::drive::identity::update::add_to_previous_balance_outcome::AddToPreviousBalanceOutcomeV0Methods;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id
    #[inline(always)]
    pub(super) fn add_to_identity_balance_v0(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
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
            platform_version,
        )?;

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

    /// Balances are stored in the balance tree under the identity's id
    /// This gets operations based on apply flag (stateful vs stateless)
    #[inline(always)]
    pub(super) fn add_to_identity_balance_operations_v0(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        let drive_version = &platform_version.drive;
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;
            Self::add_estimation_costs_for_negative_credit(
                identity_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;
        }

        let previous_balance = self
            .fetch_identity_balance_operations(
                identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance",
            )))?;

        let add_to_previous_balance = self.add_to_previous_balance(
            identity_id,
            previous_balance,
            added_balance,
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        if let Some(new_balance) = add_to_previous_balance.balance_modified() {
            drive_operations
                .push(self.update_identity_balance_operation_v0(identity_id, new_balance)?);
        }

        if let Some(new_negative_balance) =
            add_to_previous_balance.negative_credit_balance_modified()
        {
            drive_operations.push(
                self.update_identity_negative_credit_operation_v0(
                    identity_id,
                    new_negative_balance,
                ),
            );
        }

        Ok(drive_operations)
    }
}
