use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;

use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_voting_path,
    prefunded_specialized_balances_for_voting_path_vec,
};
use crate::error::identity::IdentityError;
use dpp::balances::credits::MAX_CREDITS;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{GroveDbOp, KeyInfoPath};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations to add to the specialized balance
    #[inline(always)]
    pub(super) fn add_prefunded_specialized_balance_operations_v0(
        &self,
        specialized_balance_id: Identifier,
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_prefunded_specialized_balance_update(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let path_holding_specialized_balances = prefunded_specialized_balances_for_voting_path();
        let previous_credits_in_specialized_balance = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_specialized_balances).into(),
                specialized_balance_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
        let had_previous_balance = previous_credits_in_specialized_balance.is_some();
        let new_total = previous_credits_in_specialized_balance
            .unwrap_or_default()
            .checked_add(amount)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "trying to add an amount that would overflow credits",
            )))?;
        // while i64::MAX could potentially work, best to avoid it.
        if new_total >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set prefunded specialized balance to over max credits amount (i64::MAX)",
            )));
        };
        let path_holding_total_credits_vec = prefunded_specialized_balances_for_voting_path_vec();
        let op = if had_previous_balance {
            GroveDbOp::replace_op(
                path_holding_total_credits_vec,
                specialized_balance_id.to_vec(),
                Element::new_sum_item(new_total as i64),
            )
        } else {
            GroveDbOp::insert_op(
                path_holding_total_credits_vec,
                specialized_balance_id.to_vec(),
                Element::new_sum_item(new_total as i64),
            )
        };
        drive_operations.push(GroveOperation(op));
        Ok(drive_operations)
    }
}
