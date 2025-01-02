use crate::drive::balances::{
    total_token_supply_path, total_token_supply_path_vec, total_tokens_root_supply_path,
    total_tokens_root_supply_path_vec,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::Element::Item;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    pub(super) fn remove_from_token_total_supply_v0(
        &self,
        token_id: [u8; 32],
        amount: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.remove_from_token_total_supply_add_to_operations_v0(
            token_id,
            amount,
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

    pub(super) fn remove_from_token_total_supply_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        amount: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.remove_from_token_total_supply_operations_v0(
            token_id,
            amount,
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

    pub(super) fn remove_from_token_total_supply_operations_v0(
        &self,
        token_id: [u8; 32],
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        // If we only estimate, add estimation costs
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // Add your estimation logic similar to add_to_token_total_supply if needed
            // For example (this is a placeholder method, you must implement similarly as others):
            Self::add_estimation_costs_for_token_total_supply(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let path_holding_total_token_supply = total_tokens_root_supply_path();
        let total_token_supply_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_total_token_supply).into(),
                &token_id,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Total supply for Token not found in Platform",
            )))?;
        let new_total = total_token_supply_in_platform
            .checked_sub(amount)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "trying to subtract an amount that would underflow total supply",
            )))?;
        let path_holding_total_token_supply_vec = total_tokens_root_supply_path_vec();
        let replace_op = QualifiedGroveDbOp::replace_op(
            path_holding_total_token_supply_vec,
            token_id.to_vec(),
            Item(new_total.encode_var_vec(), None),
        );
        drive_operations.push(GroveOperation(replace_op));

        Ok(drive_operations)
    }
}
