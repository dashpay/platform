use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::MAX_CREDITS;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use crate::drive::tokens::paths::token_balances_path_vec;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id
    #[allow(clippy::too_many_arguments)]
    pub(in crate::drive::tokens) fn remove_from_identity_token_balance_v0(
        &self,
        token_id: [u8; 32],
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

        let batch_operations = self.remove_from_identity_token_balance_operations_v0(
            token_id,
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
    pub(in crate::drive::tokens) fn remove_from_identity_token_balance_operations_v0(
        &self,
        token_id: [u8; 32],
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
                    "identity with token balance {} does not have the required balance to remove {}",
                    previous_balance, balance_to_remove
                ),
            )));
        }

        let balance_path = token_balances_path_vec(token_id);

        drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
            balance_path,
            identity_id.to_vec(),
            Element::new_sum_item((previous_balance - balance_to_remove) as i64),
        ));

        Ok(drive_operations)
    }
}
