use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_readiness_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn deduct_from_readiness_fund_operations_v0(
        &self,
        fund_id: Identifier,
        amount: Credits,
        reserve: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness_fund_update(
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        // The checked read: a stored fund must be a nonnegative sum item.
        let fetched = self.fetch_readiness_fund_operations(
            fund_id.to_buffer(),
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let previous_credits = if estimated_costs_only_with_layer_info.is_some() {
            // Priced without state; the widest balance lets any deduction through.
            i64::MAX as u64
        } else {
            fetched.ok_or_else(|| {
                Error::Drive(DriveError::PrefundedSpecializedBalanceDoesNotExist(
                    format!(
                        "trying to deduct from a readiness fund {} that does not exist",
                        fund_id
                    ),
                ))
            })?
        };
        let spendable = previous_credits.saturating_sub(reserve);
        if amount > spendable {
            return Err(Error::Drive(
                DriveError::PrefundedSpecializedBalanceNotEnough(spendable, amount),
            ));
        }
        let new_total = previous_credits.checked_sub(amount).ok_or(Error::Drive(
            DriveError::PrefundedSpecializedBalanceNotEnough(previous_credits, amount),
        ))?;
        let replace_op = QualifiedGroveDbOp::replace_op(
            prefunded_specialized_balances_for_readiness_path_vec(),
            fund_id.to_vec(),
            Element::new_sum_item(new_total as i64),
        );
        drive_operations.push(GroveOperation(replace_op));
        Ok(drive_operations)
    }
}
