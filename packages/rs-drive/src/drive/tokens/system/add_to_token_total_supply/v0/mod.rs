use crate::drive::balances::{total_tokens_root_supply_path, total_tokens_root_supply_path_vec};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::Element::SumItem;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_to_token_total_supply_v0(
        &self,
        token_id: [u8; 32],
        amount: TokenAmount,
        allow_first_mint: bool,
        allow_saturation: bool,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, TokenAmount), Error> {
        let mut drive_operations = vec![];

        let token_amount = self.add_to_token_total_supply_add_to_operations_v0(
            token_id,
            amount,
            allow_first_mint,
            allow_saturation,
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

        Ok((fees, token_amount))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_to_token_total_supply_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        amount: TokenAmount,
        allow_first_mint: bool,
        allow_saturation: bool,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<TokenAmount, Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let (batch_operations, token_amount) = self.add_to_token_total_supply_operations_v0(
            token_id,
            amount,
            allow_first_mint,
            allow_saturation,
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
        )?;
        Ok(token_amount)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_to_token_total_supply_operations_v0(
        &self,
        token_id: [u8; 32],
        amount: u64,
        allow_first_mint: bool,
        allow_saturation: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<LowLevelDriveOperation>, TokenAmount), Error> {
        let mut drive_operations = vec![];

        // If we only estimate, add estimation costs
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // Add your estimation logic similar to add_to_system_credits_operations_v0
            // For example:
            Self::add_estimation_costs_for_token_total_supply(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let path_holding_total_token_supply = total_tokens_root_supply_path();
        let path_holding_total_token_supply_vec = total_tokens_root_supply_path_vec();
        let total_token_supply_in_platform = self.grove_get_raw_value_u64_from_encoded_var_vec(
            (&path_holding_total_token_supply).into(),
            &token_id,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let added_amount =
            if let Some(total_token_supply_in_platform) = total_token_supply_in_platform {
                let new_total = if allow_saturation {
                    (total_token_supply_in_platform as i64).saturating_add(amount as i64)
                } else {
                    (total_token_supply_in_platform as i64)
                        .checked_add(amount as i64)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "trying to add an amount that would overflow total supply",
                        )))?
                };
                let replace_op = QualifiedGroveDbOp::replace_op(
                    path_holding_total_token_supply_vec,
                    token_id.to_vec(),
                    SumItem(new_total, None),
                );
                drive_operations.push(GroveOperation(replace_op));
                new_total as u64 - total_token_supply_in_platform
            } else if allow_first_mint {
                if amount > i64::MAX as u64 {
                    return Err(Error::Drive(DriveError::CriticalCorruptedState(
                        "amount is over max allowed in Sum Item (i64::Max)",
                    )));
                }
                let insert_op = QualifiedGroveDbOp::insert_only_op(
                    path_holding_total_token_supply_vec,
                    token_id.to_vec(),
                    SumItem(amount as i64, None),
                );
                drive_operations.push(GroveOperation(insert_op));
                amount
            } else {
                return Err(Error::Drive(DriveError::CriticalCorruptedState(
                    "Total supply for token not found in Platform",
                )));
            };

        Ok((drive_operations, added_amount))
    }
}
