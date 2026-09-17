use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_readiness_path,
    prefunded_specialized_balances_for_readiness_path_vec,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::balances::credits::MAX_CREDITS;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
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
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness_fund_update(
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(8),
            }
        };

        let path = prefunded_specialized_balances_for_readiness_path();
        let previous_credits = self.grove_get_raw_value_u64_from_encoded_var_vec(
            (&path).into(),
            fund_id.as_slice(),
            direct_query_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
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
        let path_vec = prefunded_specialized_balances_for_readiness_path_vec();
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
