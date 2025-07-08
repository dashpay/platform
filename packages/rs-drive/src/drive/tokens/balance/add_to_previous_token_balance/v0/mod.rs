use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use crate::drive::tokens::paths::token_balances_path_vec;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds specified amount of credits to identity balance
    /// This function checks for overflows and does not exceed MAX_CREDITS
    ///
    /// Balances are stored in the balance tree under the identity's id
    #[allow(clippy::too_many_arguments)]
    pub(in crate::drive::tokens) fn add_to_identity_token_balance_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        balance_to_add: Credits,
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

        let batch_operations = self.add_to_identity_token_balance_operations_v0(
            token_id,
            identity_id,
            balance_to_add,
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

    /// Adds specified amount of credits to identity balance
    /// This function checks for overflows and does not exceed MAX_CREDITS
    ///
    /// Balances are stored in the identity under key 0
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(in crate::drive::tokens) fn add_to_identity_token_balance_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        balance_to_add: TokenAmount,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_balances(
                token_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let apply = estimated_costs_only_with_layer_info.is_none();

        let previous_balance = if apply {
            self.fetch_identity_token_balance_operations(
                token_id,
                identity_id,
                apply,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
        } else {
            None // worse case is that we insert
        };

        let balance_path = token_balances_path_vec(token_id);

        if let Some(previous_balance) = previous_balance {
            // Check for overflow
            let new_balance = (previous_balance as i64)
                .checked_add(balance_to_add as i64)
                .ok_or(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                    "Overflow of total token balance".to_string(),
                ))?;

            drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
                balance_path,
                identity_id.to_vec(),
                Element::new_sum_item(new_balance),
            ));
        } else {
            if balance_to_add > i64::MAX as u64 {
                return Err(
                    ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
                        "Token balance to add over i64 max, is {}",
                        balance_to_add
                    ))
                    .into(),
                );
            }
            drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                balance_path,
                identity_id.to_vec(),
                Element::new_sum_item(balance_to_add as i64),
            ));
        }

        Ok(drive_operations)
    }
}
