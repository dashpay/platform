use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::MAX_CREDITS;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id
    pub(in crate::drive::identity::update) fn remove_from_identity_balance_v0(
        &self,
        identity_id: [u8; 32],
        balance_to_remove: Credits,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.remove_from_identity_balance_operations_v0(
            identity_id,
            balance_to_remove,
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
            previous_fee_versions,
        )?;
        Ok(fees)
    }

    /// Removes specified amount of credits from identity balance
    /// This function doesn't go below nil balance (negative balance)
    ///
    /// Balances are stored in the identity under key 0
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(in crate::drive::identity::update) fn remove_from_identity_balance_operations_v0(
        &self,
        identity_id: [u8; 32],
        balance_to_remove: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            Self::add_estimation_costs_for_negative_credit(
                identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let previous_balance = if estimated_costs_only_with_layer_info.is_none() {
            self.fetch_identity_balance_operations(
                identity_id,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance if apply is set to true",
            )))?
        } else {
            MAX_CREDITS
        };

        // we do not have enough balance
        // there is a part we absolutely need to pay for
        if balance_to_remove > previous_balance {
            return Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                format!(
                    "identity with balance {} does not have the required balance to remove {}",
                    previous_balance, balance_to_remove
                ),
            )));
        }

        drive_operations.push(self.update_identity_balance_operation_v0(
            identity_id,
            previous_balance - balance_to_remove,
        )?);

        Ok(drive_operations)
    }
}
