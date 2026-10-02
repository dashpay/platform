use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_readiness_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use dpp::balances::credits::MAX_CREDITS;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn add_readiness_fund_operations_v0(
        &self,
        fund_id: Identifier,
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        // No stored balance makes this amount valid, so an estimate refuses it as execution does.
        if amount >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set a readiness fund to over max credits amount (i64::MAX)",
            )));
        }
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness_fund_update(
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        // The checked read: a stored fund must be a nonnegative sum item.
        let previous_credits = self.fetch_readiness_fund_operations(
            fund_id.to_buffer(),
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let path_vec = prefunded_specialized_balances_for_readiness_path_vec();
        if estimated_costs_only_with_layer_info.is_some() {
            // The estimator bills a plain insert by the element's serialized size while the
            // applied write bills the fixed sum item size, so price the widest sum item.
            drive_operations.push(GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
                path_vec,
                fund_id.to_vec(),
                Element::new_sum_item(i64::MAX),
            )));
            return Ok(drive_operations);
        }
        let had_previous_balance = previous_credits.is_some();
        let new_total = previous_credits
            .unwrap_or_default()
            .checked_add(amount)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "trying to add an amount that would overflow credits",
            )))?;
        if new_total >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set a readiness fund to over max credits amount (i64::MAX)",
            )));
        };
        let op = if had_previous_balance {
            QualifiedGroveDbOp::replace_op(
                path_vec,
                fund_id.to_vec(),
                Element::new_sum_item(new_total as i64),
            )
        } else {
            QualifiedGroveDbOp::insert_or_replace_op(
                path_vec,
                fund_id.to_vec(),
                Element::new_sum_item(new_total as i64),
            )
        };
        drive_operations.push(GroveOperation(op));
        Ok(drive_operations)
    }
}
