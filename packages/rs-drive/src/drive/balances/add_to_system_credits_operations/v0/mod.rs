use crate::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::system::{misc_path, misc_path_vec};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::GroveOperation;

use dpp::version::PlatformVersion;
use grovedb::batch::{GroveDbOp, KeyInfoPath};
use grovedb::Element::Item;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    /// The operations to add to system credits
    #[inline(always)]
    pub(super) fn add_to_system_credits_operations_v0(
        &self,
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_total_system_credits_update(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let path_holding_total_credits = misc_path();
        let total_credits_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_total_credits).into(),
                TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits not found in Platform",
            )))?;
        let new_total = total_credits_in_platform
            .checked_add(amount)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "trying to add an amount that would overflow credits",
            )))?;
        let path_holding_total_credits_vec = misc_path_vec();
        let replace_op = GroveDbOp::replace_op(
            path_holding_total_credits_vec,
            TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec(),
            Item(new_total.encode_var_vec(), None),
        );
        drive_operations.push(GroveOperation(replace_op));
        Ok(drive_operations)
    }
}
