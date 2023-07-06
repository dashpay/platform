use std::collections::HashMap;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use grovedb::batch::KeyInfoPath;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use crate::fee::calculate_fee;
use dpp::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::balances::balance_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::error::identity::IdentityError;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {

/// Balances are stored in the balance tree under the identity's id
pub(in crate::drive::identity::update) fn remove_from_identity_balance_v0(
    &self,
    identity_id: [u8; 32],
    balance_to_remove: Credits,
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

    let batch_operations = self.remove_from_identity_balance_operations_v0(
        identity_id,
        balance_to_remove,
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

    let previous_balance = if estimated_costs_only_with_layer_info.is_none() {
        self.fetch_identity_balance_operations(
            identity_id,
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            &mut drive_operations,
            drive_version,
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

    drive_operations.push(self.update_identity_balance_operation(
        identity_id,
        previous_balance - balance_to_remove,
        drive_version,
    )?);

    Ok(drive_operations)
}
}